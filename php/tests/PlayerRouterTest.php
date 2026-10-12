<?php

declare(strict_types=1);

namespace Lingara\Embed\Tests;

use GuzzleHttp\Psr7\HttpFactory;
use Lingara\Embed\PlayerRouter;
use Lingara\Embed\RoutedEvent;
use Lingara\Events\Generated\LessonPlanReady;
use Nyholm\Psr7\Factory\Psr17Factory;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;
use Psr\Http\Message\ServerRequestFactoryInterface;
use Psr\Http\Message\ServerRequestInterface;
use Psr\Http\Message\StreamFactoryInterface;

final class PlayerRouterTest extends TestCase
{
    private const KEY = 'player-router-test-key-32-bytes!';
    private const SUBJECT = 'lgr_sub_conformance';

    /** @var list<RoutedEvent> */
    private array $seen = [];
    /** @var list<\Throwable> */
    private array $errors = [];

    /** @return iterable<string, array{ServerRequestFactoryInterface&StreamFactoryInterface}> */
    public static function stacks(): iterable
    {
        yield 'nyholm/psr7' => [new Psr17Factory()];
        yield 'guzzlehttp/psr7' => [new HttpFactory()];
    }

    /**
     * 1.10.26af AC16: on both PSR-7 stacks, a tampered body is 401 with no
     * directory lookup and no onEvent; a valid delivery reaches onEvent with
     * the remembered player_ref (204); an unknown subject reaches it with
     * none; a valid signature over a body that is not JSON is 400; a raising
     * handler and a raising directory are each 500 with onError called.
     * The body is valid-single-signature's from webhook-signatures.json,
     * signed here at the current time.
     */
    #[DataProvider('stacks')]
    public function testHandleVerifiesBeforeResolving(ServerRequestFactoryInterface&StreamFactoryInterface $psr7): void
    {
        $body = self::vectorBody();
        $directory = new CountingDirectory();
        $directory->remember('guild/42', self::SUBJECT);
        $router = new PlayerRouter($directory, 'lgr_whsec_' . base64_encode(self::KEY), $this->recordError(...));

        $tampered = self::request($psr7, str_replace('night market', 'night markEt', $body), self::sign($body));
        self::assertSame(401, $router->handle((string) $tampered->getBody(), $tampered, $this->record(...)));
        self::assertSame(0, $directory->lookups, 'a forged delivery never reaches the directory');
        self::assertSame([], $this->seen);

        $valid = self::request($psr7, $body, self::sign($body));
        self::assertSame(204, $router->handle((string) $valid->getBody(), $valid, $this->record(...)));
        self::assertSame('guild/42', $this->seen(0)->playerRef);
        self::assertInstanceOf(LessonPlanReady::class, $this->seen(0)->event);

        $directory->forget('guild/42');
        self::assertSame(204, $router->handle($body, $valid, $this->record(...)));
        self::assertNull($this->seen(1)->playerRef);

        $notJson = self::request($psr7, 'not json', self::sign('not json'));
        self::assertSame(400, $router->handle('not json', $notJson, $this->record(...)));
        self::assertSame([], $this->errors);

        $raise = static fn(): never => throw new \RuntimeException('the handler failed');
        self::assertSame(500, $router->handle($body, $valid, $raise));
        $directory->throws = true;
        self::assertSame(500, $router->handle($body, $valid, $this->record(...)));
        self::assertCount(2, $this->errors);
        self::assertCount(2, $this->seen, 'the raising directory reached no handler');
    }

    /**
     * 1.10.26af AC16: with no onError, a 500's cause goes to error_log(),
     * and plain array headers are taken as well as a PSR-7 message.
     */
    public function testDefaultOnErrorIsErrorLog(): void
    {
        $log = (string) tempnam(sys_get_temp_dir(), 'lgr-embed-log');
        $previous = ini_set('error_log', $log);
        try {
            $body = self::vectorBody();
            $router = new PlayerRouter(new CountingDirectory(), 'lgr_whsec_' . base64_encode(self::KEY));
            $status = $router->handle($body, self::sign($body), static fn(): never => throw new \RuntimeException('boom'));
            self::assertSame(500, $status);
            self::assertStringContainsString('lingara webhook: the event handler failed: RuntimeException: boom', (string) file_get_contents($log));
        } finally {
            ini_set('error_log', (string) $previous);
            @unlink($log);
        }
    }

    private function seen(int $n): RoutedEvent
    {
        return $this->seen[$n] ?? self::fail("onEvent saw no delivery {$n}");
    }

    private function record(RoutedEvent $routed): void
    {
        $this->seen[] = $routed;
    }

    private function recordError(\Throwable $error): void
    {
        $this->errors[] = $error;
    }

    private static function vectorBody(): string
    {
        $file = json_decode((string) file_get_contents(__DIR__ . '/../../conformance/vectors/webhook-signatures.json'), true, 512, JSON_THROW_ON_ERROR);
        self::assertIsArray($file);
        self::assertIsArray($file['vectors']);
        foreach ($file['vectors'] as $vector) {
            if (is_array($vector) && $vector['name'] === 'valid-single-signature' && is_string($vector['body'])) {
                return $vector['body'];
            }
        }
        self::fail('webhook-signatures.json has no valid-single-signature vector');
    }

    /** @return array<string, string> Standard Webhooks headers over `body`, at the current time */
    private static function sign(string $body): array
    {
        $id = 'lgr_evt_conformance1';
        $timestamp = (string) time();
        $signature = base64_encode(hash_hmac('sha256', "{$id}.{$timestamp}.{$body}", self::KEY, true));
        return ['webhook-id' => $id, 'webhook-timestamp' => $timestamp, 'webhook-signature' => "v1,{$signature}"];
    }

    /** @param array<string, string> $headers */
    private static function request(ServerRequestFactoryInterface&StreamFactoryInterface $psr7, string $body, array $headers): ServerRequestInterface
    {
        $request = $psr7->createServerRequest('POST', 'https://game.example/lingara/webhook')->withBody($psr7->createStream($body));
        foreach ($headers as $name => $value) {
            $request = $request->withHeader($name, $value);
        }
        return $request;
    }
}
