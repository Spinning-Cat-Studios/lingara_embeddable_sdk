<?php

declare(strict_types=1);

namespace Lingara\Embed;

use Lingara\MintedToken;

/**
 * The handoff body (ADR 1.10.26af D6, ES6): exactly
 * `{"token":"lgr_et_…","expires_at":"…"}`, the shape every player kit and
 * the widget read. This is the one place the kit exposes the token on
 * purpose.
 */
final class TokenHandoff
{
    private const JSON = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_LINE_TERMINATORS | JSON_THROW_ON_ERROR;

    /**
     * The JSON your token route returns to the player. `expires_at` is the
     * server's string, which the library keeps, copied byte for byte;
     * `expires_in` and `subject` are not forwarded.
     */
    public static function body(MintedToken $t): string
    {
        return self::handoffBody($t->token->exposeSecret(), $t->expiresAt);
    }

    /**
     * The pure body over two strings, held to token-handoff.json: compact,
     * these two keys in this order, `/` and non-ASCII unescaped.
     *
     * @internal
     */
    public static function handoffBody(#[\SensitiveParameter] string $token, string $expiresAt): string
    {
        return '{"token":' . json_encode($token, self::JSON) . ',"expires_at":' . json_encode($expiresAt, self::JSON) . '}';
    }
}
