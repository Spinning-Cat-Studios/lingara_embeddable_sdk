<?php

declare(strict_types=1);

namespace Lingara\Embed;

use Lingara\MintedToken;
use Lingara\Model\UpdateAvailable;

/**
 * The handoff body (ADR 1.10.26af D6, ES6): exactly
 * `{"token":"lgr_et_…","expires_at":"…"}`, the shape every player kit and
 * the widget read, plus `update_available` when the mint answered one (ADR
 * 31.10.26f D1). This is the one place the kit exposes the token on
 * purpose.
 */
final class TokenHandoff
{
    private const JSON = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_LINE_TERMINATORS | JSON_THROW_ON_ERROR;

    /**
     * The JSON your token route returns to the player. `expires_at` is the
     * server's string, which the library keeps, copied byte for byte;
     * `expires_in` and `subject` are not forwarded. `update_available`, the
     * mint's notice that the player's declared build is behind, deprecated
     * or withdrawn, is forwarded when there is one.
     */
    public static function body(MintedToken $t): string
    {
        return self::handoffBody($t->token->exposeSecret(), $t->expiresAt, $t->updateAvailable);
    }

    /**
     * The pure body, held to token-handoff.json: compact, `token`,
     * `expires_at` and, only when given, `update_available` in this order,
     * `/` and non-ASCII unescaped. Without a notice the body is exactly the
     * two-key body.
     *
     * @internal
     */
    public static function handoffBody(#[\SensitiveParameter] string $token, string $expiresAt, ?UpdateAvailable $update = null): string
    {
        $body = '{"token":' . json_encode($token, self::JSON) . ',"expires_at":' . json_encode($expiresAt, self::JSON);
        return $body . ($update === null ? '' : ',"update_available":' . self::update($update)) . '}';
    }

    /**
     * The notice from its typed value, never a raw map: `latest`,
     * `deprecated`, `message`, in this order, all three always written and
     * a JSON null kept.
     */
    private static function update(UpdateAvailable $u): string
    {
        return '{"latest":' . json_encode($u->getLatest(), self::JSON)
            . ',"deprecated":' . json_encode($u->getDeprecated(), self::JSON)
            . ',"message":' . json_encode($u->getMessage(), self::JSON) . '}';
    }
}
