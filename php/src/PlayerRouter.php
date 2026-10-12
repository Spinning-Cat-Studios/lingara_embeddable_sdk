<?php

declare(strict_types=1);

namespace Lingara\Embed;

use Lingara\Events\Event;
use Lingara\Events\VerificationException;
use Lingara\Events\Webhook;
use Psr\Http\Message\MessageInterface;

/**
 * Verifies Lingara's webhooks and resolves each to a player (ADR 1.10.26af
 * D8): the library's verifier on the raw body first, then the directory, so
 * a forged event never reaches your store.
 *
 *     $router = new PlayerRouter($directory, getenv('LINGARA_WEBHOOK_SECRET'));
 *     http_response_code($router->handle(file_get_contents('php://input'), getallheaders(), $onEvent));
 */
final class PlayerRouter
{
    private readonly Webhook $webhook;
    /** @var \Closure(\Throwable): void|null */
    private readonly ?\Closure $onError;

    /**
     * The verifier is built here, once, so a malformed secret fails at
     * startup with the library's \InvalidArgumentException.
     *
     * @param string|list<string>                $secrets one `lgr_whsec_` secret, or each live one during a rotation
     * @param (callable(\Throwable): void)|null $onError where a 500's cause goes; by default error_log()
     *
     * @throws \InvalidArgumentException
     */
    public function __construct(
        private readonly PlayerDirectory $directory,
        #[\SensitiveParameter]
        string|array $secrets,
        ?callable $onError = null,
    ) {
        $this->webhook = new Webhook($secrets);
        $this->onError = $onError === null ? null : \Closure::fromCallable($onError);
    }

    /**
     * Verifies the **raw** body, then looks its subject up.
     *
     * @param array<string, string|list<string>>|MessageInterface $headers the request's headers, or any PSR-7 request
     *
     * @throws VerificationException before the directory is consulted
     */
    public function route(string $body, array|MessageInterface $headers): RoutedEvent
    {
        $event = $this->webhook->verify($body, $headers);
        return new RoutedEvent($event, $this->directory->playerRefFor(self::subject($event)));
    }

    /**
     * route(), then `$onEvent($routed)`, answered as the status your route
     * sends with an empty body: 204 handled, 401 a bad signature, 400 a
     * malformed payload, 500 the directory or the handler threw (the cause
     * goes to onError, never with the body, a signature or a secret).
     * Lingara redelivers every non-2xx. `$onEvent` runs inline and delivery
     * expects a 2xx within 10 s: a slow handler enqueues and returns.
     *
     * @param array<string, string|list<string>>|MessageInterface $headers as route()
     * @param callable(RoutedEvent): mixed                         $onEvent
     */
    public function handle(string $body, array|MessageInterface $headers, callable $onEvent): int
    {
        try {
            $routed = $this->route($body, $headers);
        } catch (VerificationException $e) {
            return $e->reason() === VerificationException::MALFORMED_PAYLOAD ? 400 : 401;
        } catch (\Throwable $e) {
            return $this->failed('the player directory', $e);
        }
        try {
            $onEvent($routed);
        } catch (\Throwable $e) {
            return $this->failed('the event handler', $e);
        }
        return 204;
    }

    /** @return array<string, mixed> */
    public function __debugInfo(): array
    {
        return ['webhook' => $this->webhook, 'directory' => $this->directory::class];
    }

    /** @return array<string, mixed> */
    public function __serialize(): array
    {
        throw new \LogicException('a PlayerRouter cannot be serialized');
    }

    /** @param array<mixed> $data */
    public function __unserialize(array $data): void
    {
        throw new \LogicException('a PlayerRouter cannot be unserialized');
    }

    private function failed(string $what, \Throwable $error): int
    {
        if ($this->onError !== null) {
            ($this->onError)($error);
        } else {
            error_log("lingara webhook: {$what} failed: " . $error::class . ': ' . $error->getMessage());
        }
        return 500;
    }

    /** Every event arm, UnknownEvent included, holds the envelope's `subject`. */
    private static function subject(Event $event): string
    {
        $subject = property_exists($event, 'subject') ? $event->subject : null;
        return is_string($subject) ? $subject : '';
    }
}
