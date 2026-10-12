<?php

declare(strict_types=1);

namespace Lingara\Embed\ConformanceEs;

use Lingara\Embed\PlayerRouter;
use Lingara\Embed\RoutedEvent;
use Lingara\Events\UnknownEvent;
use Psr\Http\Message\ServerRequestInterface;

/**
 * The webhook receiver's half. Delivery `n` of the armed case is answered
 * through a PlayerRouter built from that item's `router.secrets` (absent:
 * `[primary]`), over the case's directory rows, whose onEvent records what
 * it saw and raises when the item's `handler` is `raise`.
 */
final class Receiver
{
    /**
     * @param list<mixed>           $deliver
     * @param array<string, string> $rows
     */
    public static function arm(SharedState $state, array $deliver, array $rows): void
    {
        $armed = [
            'deliver' => $deliver, 'directory' => $rows, 'seen' => array_fill(0, count($deliver), null),
            'received' => 0, 'answered' => 0, 'lookups' => 0,
        ];
        $state->update(static fn(): array => $armed);
    }

    /** @param array<string, string> $rows */
    public static function syncDirectory(SharedState $state, array $rows): void
    {
        $state->update(static fn(?array $armed): ?array => $armed === null ? null : ['directory' => $rows] + $armed);
    }

    /** The router delivery `item` is answered through. */
    public static function router(SharedState $state, mixed $item, ?Recording $directory = null): PlayerRouter
    {
        $named = explode(',', Harness::env('LINGARA_EMBED_WEBHOOK_SECRETS'));
        $secrets = ['primary' => $named[0], 'secondary' => $named[1] ?? ''];
        $router = is_array($item) && is_array($item['router'] ?? null) ? $item['router'] : [];
        $names = is_array($router['secrets'] ?? null) ? $router['secrets'] : ['primary'];
        $chosen = [];
        foreach ($names as $name) {
            $chosen[] = is_string($name) && isset($secrets[$name]) ? $secrets[$name] : throw new \RuntimeException('no secret named ' . json_encode($name));
        }
        $onError = static fn(\Throwable $e) => $state->sink('on_error: ' . $e::class . ': ' . $e->getMessage());
        return new PlayerRouter($directory ?? new Recording(), $chosen, $onError);
    }

    /** One delivery, answered with the router's status (409 when nothing is armed). */
    public static function deliver(SharedState $state, ServerRequestInterface $request): int
    {
        $claimed = $state->update(static function (?array $armed, mixed &$out): ?array {
            if ($armed !== null) {
                $out = [$armed['received'], $armed['deliver'][$armed['received']] ?? null, $armed['directory']];
                $armed['received']++;
            }
            return $armed;
        });
        if (!is_array($claimed)) {
            return 409;
        }
        [$n, $item, $rows] = $claimed;
        $directory = new Recording(Judge::stringMap($rows));
        $seen = null;
        try {
            $raise = is_array($item) && ($item['handler'] ?? null) === 'raise';
            $router = self::router($state, $item, $directory);
            $status = $router->handle((string) $request->getBody(), $request, static function (RoutedEvent $routed) use (&$seen, $raise): void {
                $seen = self::seen($routed);
                if ($raise) {
                    throw new \RuntimeException('the conformance handler raised, as the case asked');
                }
            });
        } catch (\Throwable $e) {
            $state->sink('harness: ' . $e->getMessage());
            $status = 500;
        }
        self::answered($state, is_int($n) ? $n : 0, $seen, $directory->lookups);
        return $status;
    }

    /** @return array{player_ref: string|null, event_id: string, unknown: bool} */
    private static function seen(RoutedEvent $routed): array
    {
        $id = property_exists($routed->event, 'id') ? $routed->event->id : null;
        return ['player_ref' => $routed->playerRef, 'event_id' => is_string($id) ? $id : '', 'unknown' => $routed->event instanceof UnknownEvent];
    }

    /** @param array{player_ref: string|null, event_id: string, unknown: bool}|null $seen */
    private static function answered(SharedState $state, int $n, ?array $seen, int $lookups): void
    {
        $state->update(static function (?array $armed) use ($n, $seen, $lookups): ?array {
            if ($armed !== null) {
                $armed['seen'][$n] = $seen;
                $armed['answered']++;
                $armed['lookups'] += $lookups;
            }
            return $armed;
        });
    }
}
