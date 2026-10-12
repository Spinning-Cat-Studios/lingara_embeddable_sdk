<?php

declare(strict_types=1);

namespace Lingara\Embed\Tests;

use PHPUnit\Framework\TestCase;

final class ComposerTest extends TestCase
{
    /**
     * 1.10.26af AC24: composer.json's `require` is php and the lingara
     * library (at the floor in LIBRARY_FLOOR, as a caret) only: no HTTP,
     * JSON or crypto package, and the PSR-7 stacks the tests and harness use
     * stay in `require-dev`. It has no `version` field.
     */
    public function testRequireIsTheLibraryOnly(): void
    {
        $manifest = json_decode((string) file_get_contents(__DIR__ . '/../composer.json'), true, 512, JSON_THROW_ON_ERROR);
        self::assertIsArray($manifest);
        self::assertIsArray($manifest['require']);
        $require = array_keys($manifest['require']);
        sort($require);
        self::assertSame(['php', 'spinningcatstudios/lingara'], $require);
        $floor = trim((string) file_get_contents(__DIR__ . '/../../LIBRARY_FLOOR'));
        self::assertSame("^{$floor}", $manifest['require']['spinningcatstudios/lingara']);
        self::assertSame('>=8.2', $manifest['require']['php']);
        self::assertIsArray($manifest['require-dev']);
        self::assertArrayHasKey('nyholm/psr7', $manifest['require-dev']);
        self::assertArrayHasKey('guzzlehttp/psr7', $manifest['require-dev']);
        self::assertArrayNotHasKey('version', $manifest);
        self::assertSame('spinningcatstudios/lingara-embed', $manifest['name']);
        self::assertIsArray($manifest['autoload']);
        self::assertSame(['Lingara\\Embed\\' => 'src/'], $manifest['autoload']['psr-4'] ?? null);
    }
}
