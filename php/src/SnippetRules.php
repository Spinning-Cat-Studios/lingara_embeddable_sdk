<?php

declare(strict_types=1);

namespace Lingara\Embed;

/**
 * ES4's option rules (ADR 1.10.26af D7), one method per row of the
 * contract's table. Every pattern is ASCII-only and anchored with `\z`
 * (PHP's `$` would accept a trailing newline).
 *
 * @internal
 */
final class SnippetRules
{
    /** Unicode category Cc. */
    private const CONTROL = '/[\x{0}-\x{1f}\x{7f}-\x{9f}]/u';
    /** The contract's whitespace set (Unicode White_Space beyond Cc). */
    private const SPACE = '/[\x{20}\x{85}\x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}]/u';
    private const ORIGIN = '#^(https://[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*(:[0-9]{1,5})?|http://(localhost|127\.0\.0\.1)(:[0-9]{1,5})?)/?\z#';

    public static function planId(string $s): bool
    {
        return self::matches('/^[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}\z/', $s);
    }

    public static function surface(string $s): bool
    {
        return self::matches('/^[a-z][a-z0-9-]{0,31}\z/', $s);
    }

    public static function theme(string $s): bool
    {
        return in_array($s, ['light', 'dark', 'auto'], true);
    }

    public static function locale(string $s): bool
    {
        return self::matches('/^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}\z/', $s);
    }

    public static function lingaraOrigin(string $s): bool
    {
        return self::matches(self::ORIGIN, $s);
    }

    /** 1–80 Unicode scalar values, none of them a control character. */
    public static function title(string $s): bool
    {
        $count = preg_match_all('/./su', $s);
        return is_int($count) && $count >= 1 && $count <= 80 && preg_match(self::CONTROL, $s) === 0;
    }

    public static function id(string $s): bool
    {
        return self::matches('/^[A-Za-z][A-Za-z0-9_-]{0,63}\z/', $s);
    }

    /** A path that starts `/` but not `//`, or an absolute `https://` URL. */
    public static function tokenEndpoint(string $s): bool
    {
        $shape = (str_starts_with($s, '/') && !str_starts_with($s, '//')) || str_starts_with($s, 'https://');
        return $shape && self::urlText($s);
    }

    public static function nonce(string $s): bool
    {
        return self::matches('#^[A-Za-z0-9+/=_-]{8,128}\z#', $s);
    }

    public static function elementSrc(string $s): bool
    {
        return str_starts_with($s, 'https://') && self::urlText($s);
    }

    /** At most 2048 bytes of valid UTF-8, with no control character or whitespace. */
    private static function urlText(string $s): bool
    {
        return strlen($s) <= 2048 && preg_match(self::CONTROL, $s) === 0 && preg_match(self::SPACE, $s) === 0;
    }

    private static function matches(string $pattern, string $s): bool
    {
        return preg_match($pattern, $s) === 1;
    }
}
