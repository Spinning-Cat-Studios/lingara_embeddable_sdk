<?php

declare(strict_types=1);

namespace Lingara\Embed\ConformanceEs;

use Lingara\MintedToken;

/**
 * The comparisons only the kit's caller can make (ES CONTRACT.md, *What a
 * harness reports*), each JSON-equal after dropping nulls, each a
 * `<id>: fail <key>: …` line when it differs.
 */
final class Judge
{
    private const SECRET_PREFIXES = ['lgr_et_', 'lgr_cs_', 'lgr_whsec_'];
    private const KEYS = ['minted', 'handoff', 'error', 'directory', 'directory_lookups'];

    /** @param array<string, mixed> $armed the shared state after the last delivery */
    public function __construct(
        private readonly string $id,
        private readonly Recording $directory,
        private readonly array $armed,
    ) {}

    /** @return array<string, string> */
    public static function stringMap(mixed $value): array
    {
        $map = [];
        foreach (is_array($value) ? $value : [] as $key => $item) {
            $map[(string) $key] = is_string($item) ? $item : '';
        }
        return $map;
    }

    /**
     * `expect.minted`, `.handoff`, `.error`, `.directory` and `.directory_lookups`.
     *
     * @param array<mixed> $expect
     *
     * @return list<string>
     */
    public function expectations(array $expect, Outcome $out): array
    {
        $found = [];
        foreach (self::KEYS as $key) {
            $have = $this->got($key, $out);
            $want = array_key_exists($key, $expect) ? self::canonical($expect[$key]) : null;
            // A key the case names must match; an error it did not expect is a mismatch too.
            $differs = array_key_exists($key, $expect) ? $have !== $want : $key === 'error' && $have !== null;
            if ($differs) {
                $found[] = "{$this->id}: fail {$key}: got " . self::show($have) . ', expected ' . self::show($want);
            }
        }
        return $found;
    }

    /**
     * Each delivery's `expect.handler` against what its onEvent saw.
     *
     * @param list<mixed> $deliver
     *
     * @return list<string>
     */
    public function handlers(array $deliver): array
    {
        $found = [];
        foreach ($deliver as $n => $item) {
            $want = is_array($item) && is_array($item['expect'] ?? null) ? ($item['expect']['handler'] ?? null) : null;
            $got = $this->seen($n);
            if (!self::handlerMatches($want, $got)) {
                $found[] = "{$this->id}: fail handler: deliver[{$n}] saw " . self::show($got) . ', expected ' . self::show($want);
            }
        }
        return $found;
    }

    /**
     * The player each delivery's handler saw, null for one never called.
     *
     * @return list<string|null>
     */
    public function handlerPlayers(int $count): array
    {
        $players = [];
        for ($n = 0; $n < $count; $n++) {
            $player = $this->seen($n)['player_ref'] ?? null;
            $players[] = is_string($player) ? $player : null;
        }
        return $players;
    }

    /**
     * ES5: no rendering of a kit object, and no line in the sink, carries a
     * secret prefix.
     *
     * @param list<object> $objects the server and a router
     *
     * @return list<string>
     */
    public function renderings(SharedState $state, array $objects, Outcome $out): array
    {
        $renderings = [];
        foreach ([...$objects, $this->directory->inner, $out->minted] as $object) {
            $renderings[] = print_r($object, true);
            ob_start();
            var_dump($object);
            $renderings[] = (string) ob_get_clean();
        }
        $renderings[] = var_export($out->minted, true);
        array_push($renderings, ...$state->takeSink());
        $leaks = array_filter($renderings, static function (string $r): bool {
            foreach (self::SECRET_PREFIXES as $prefix) {
                if (str_contains($r, $prefix)) {
                    return true;
                }
            }
            return false;
        });
        return $leaks === [] ? [] : ["{$this->id}: fail render: " . count($leaks) . ' rendering(s) carry a secret prefix'];
    }

    private function got(string $key, Outcome $out): mixed
    {
        $lookups = $this->directory->lookups + (is_int($this->armed['lookups'] ?? null) ? $this->armed['lookups'] : 0);
        return self::canonical(match ($key) {
            'minted' => $out->minted === null ? null : self::wire($out->minted),
            'handoff' => $out->handoff,
            'error' => $out->error,
            'directory' => $this->directory->snapshot(),
            default => $lookups,
        });
    }

    /** @return array<mixed>|null */
    private function seen(int $n): ?array
    {
        $seen = is_array($this->armed['seen'] ?? null) ? ($this->armed['seen'][$n] ?? null) : null;
        return is_array($seen) ? $seen : null;
    }

    /** @param array<mixed>|null $got */
    private static function handlerMatches(mixed $want, ?array $got): bool
    {
        if ($want === 'not_called' || $got === null) {
            return $want === 'not_called' && $got === null;
        }
        if (!is_array($want)) {
            return false;
        }
        foreach (['player_ref', 'event_id', 'unknown'] as $key) {
            if (array_key_exists($key, $want) && ($key !== 'player_ref' || $want[$key] !== null || $got[$key] !== null) && $want[$key] !== $got[$key]) {
                return false;
            }
        }
        return true;
    }

    /**
     * The six fields in wire form, the token read through its accessor.
     *
     * @return array<string, mixed>
     */
    private static function wire(MintedToken $t): array
    {
        return [
            'token' => $t->token->exposeSecret(), 'expires_at' => $t->expiresAt, 'expires_in' => $t->expiresIn,
            'subject' => $t->subject, 'scopes' => $t->scopes, 'account_linked' => $t->accountLinked,
        ];
    }

    /** Nulls dropped from every object, at any depth, and object keys sorted. */
    private static function canonical(mixed $value): mixed
    {
        if (!is_array($value)) {
            return $value;
        }
        $out = [];
        foreach ($value as $key => $item) {
            if ($item !== null || array_is_list($value)) {
                $out[$key] = self::canonical($item);
            }
        }
        if (!array_is_list($value)) {
            ksort($out, SORT_STRING);
        }
        return $out;
    }

    private static function show(mixed $value): string
    {
        return $value === null ? 'nothing' : (string) json_encode($value, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    }
}
