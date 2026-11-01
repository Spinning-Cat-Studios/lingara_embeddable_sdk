<?php

declare(strict_types=1);

namespace Lingara\Embed\Tests;

use Lingara\Embed\SnippetException;
use Lingara\Embed\SnippetOptions;
use Lingara\Embed\TokenHandoff;
use Lingara\Embed\Version;
use Lingara\Embed\WidgetSnippet;
use Lingara\Model\UpdateAvailable;
use PHPUnit\Framework\TestCase;

final class VectorsTest extends TestCase
{
    private const VECTORS = __DIR__ . '/../../conformance/vectors/';

    /**
     * 1.10.26af AC8: every case of conformance/vectors/snippet.json renders
     * exactly its `html` ({{version}} replaced by Version::VERSION) or is
     * refused with SnippetException naming its `field`; and every case of
     * token-handoff.json gives exactly its `body` through handoffBody(), its
     * `update_available` decoded to the library's typed UpdateAvailable
     * first, so key order is the encoder's.
     */
    // 31.10.26f AC4
    public function testEverySnippetAndHandoffVector(): void
    {
        $snippets = self::read('snippet.json');
        self::assertIsArray($snippets['cases']);
        self::assertNotEmpty($snippets['cases']);
        foreach ($snippets['cases'] as $case) {
            self::assertIsArray($case);
            self::snippetCase($case);
        }
        $handoffs = self::read('token-handoff.json');
        self::assertNotEmpty($handoffs);
        $notices = 0;
        foreach ($handoffs as $case) {
            self::assertIsArray($case);
            self::assertIsArray($case['input']);
            [$token, $expiresAt, $body] = [$case['input']['token'], $case['input']['expires_at'], $case['body']];
            self::assertIsString($token);
            self::assertIsString($expiresAt);
            $notice = self::notice($case['input']);
            $notices += $notice === null ? 0 : 1;
            self::assertSame($body, TokenHandoff::handoffBody($token, $expiresAt, $notice), (string) json_encode($case['name']));
        }
        self::assertGreaterThanOrEqual(3, $notices, "31.10.26f's three vectors are here");
    }

    /**
     * The vector's `update_available` as the library's typed value, the
     * one MintedToken carries; null when the vector has none.
     *
     * @param array<mixed> $input
     */
    private static function notice(array $input): ?UpdateAvailable
    {
        if (!array_key_exists('update_available', $input)) {
            return null;
        }
        $raw = $input['update_available'];
        self::assertIsArray($raw);
        self::assertTrue(array_key_exists('latest', $raw) && array_key_exists('message', $raw), 'all three keys');
        self::assertIsBool($raw['deprecated']);
        return new UpdateAvailable(['latest' => $raw['latest'], 'deprecated' => $raw['deprecated'], 'message' => $raw['message']]);
    }

    /** @param array<mixed> $case */
    private static function snippetCase(array $case): void
    {
        $name = is_string($case['name']) ? $case['name'] : '';
        self::assertIsArray($case['options']);
        self::assertIsArray($case['expect']);
        $options = [];
        foreach ($case['options'] as $key => $value) {
            self::assertIsString($value);
            $options[(string) $key] = $value;
        }
        try {
            $html = WidgetSnippet::render(new SnippetOptions(...$options));
        } catch (SnippetException $e) {
            self::assertSame($case['expect']['refused'] ?? null, $e->field, "{$name}: refused");
            return;
        }
        self::assertArrayHasKey('html', $case['expect'], "{$name}: rendered, but the vector expects a refusal");
        self::assertIsString($case['expect']['html']);
        self::assertSame(str_replace('{{version}}', Version::VERSION, $case['expect']['html']), $html, $name);
    }

    /** @return array<mixed> */
    private static function read(string $file): array
    {
        $decoded = json_decode((string) file_get_contents(self::VECTORS . $file), true, 512, JSON_THROW_ON_ERROR);
        self::assertIsArray($decoded);
        return $decoded;
    }
}
