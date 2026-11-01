<?php

declare(strict_types=1);

namespace Lingara\Embed;

use Lingara\Client;
use Lingara\MintedToken;
use Lingara\Model\EmbedTokenRequest;
use Psr\Http\Message\MessageInterface;

/**
 * Mint and delete (ADR 1.10.26af D4, D9): one library call each, the
 * directory written only after a success. There is no cache: every
 * mintToken() is one POST, so a token deletePlayer() revoked on another
 * instance is never handed out again. A failed call throws the library's
 * own exception, unchanged.
 *
 *     $server = new EmbedServer($client, $directory);
 *     $minted = $server->mintToken($player->id, origin: 'https://game.example');
 */
final class EmbedServer
{
    /**
     * @param Client               $client    built with your client credentials; its
     *                                        token cache, retries and deprecation hook are the library's
     * @param PlayerDirectory|null $directory records each mint's subject and forgets a deleted player
     */
    public function __construct(
        private readonly Client $client,
        private readonly ?PlayerDirectory $directory = null,
    ) {}

    /**
     * Mints a token for one player: one `POST /v1/embed/tokens`, then
     * `remember($playerRef, $minted->subject)`. `scopes`, `origin` and
     * `embedVersion` are sent only when given; `origin` is your page's
     * origin, one of your client's embed origins (a game's server mints
     * without one), and `embedVersion` is the player build's declared
     * version, read with embedVersionFrom(). Lingara answers an outdated one
     * with `$minted->updateAvailable`. On a failure nothing is remembered.
     *
     * @param list<string>|null $scopes
     *
     * @throws \Lingara\Exception\LingaraException
     * @throws \InvalidArgumentException for a value the request model refuses before anything is sent
     */
    public function mintToken(string $playerRef, ?array $scopes = null, ?string $origin = null, ?string $embedVersion = null): MintedToken
    {
        // `origin` and `embed_version` go in through the constructor, not
        // their setters: the library's setters call an
        // ObjectSerializer::toString() that does not exist, so they throw
        // \Error for any non-null value. Lingara checks both values either
        // way (B1; ADR 31.10.26f D1: the kit does not validate the version).
        $fields = array_filter(['origin' => $origin, 'embed_version' => $embedVersion], static fn (?string $v): bool => $v !== null);
        $request = (new EmbedTokenRequest($fields === [] ? null : $fields))->setPlayerRef($playerRef);
        if ($scopes !== null) {
            $request->setScopes($scopes);
        }
        $minted = $this->client->createEmbedToken($request)->value;
        $this->directory?->remember($playerRef, $minted->subject);
        return $minted;
    }

    /**
     * The `Lingara-Embed-Version` header a player kit sends to your token
     * route, read case-insensitively and trimmed (ADR 31.10.26f D1): pass it
     * to mintToken() as `embedVersion`. A missing or blank header is null, so
     * the mint sends no key. The value is not validated: Lingara is the
     * authority.
     *
     * @param array<string, string|list<string>>|MessageInterface $headers getallheaders(), or any PSR-7 request, as PlayerRouter::handle()
     */
    public static function embedVersionFrom(array|MessageInterface $headers): ?string
    {
        $name = 'Lingara-Embed-Version';
        if ($headers instanceof MessageInterface) {
            $found = $headers->hasHeader($name) ? $headers->getHeaderLine($name) : null;
        } else {
            $found = null;
            foreach ($headers as $key => $value) {
                if (strcasecmp((string) $key, $name) === 0) {
                    $found = is_array($value) ? implode(', ', $value) : $value;
                }
            }
        }
        $trimmed = $found === null ? '' : trim($found);
        return $trimmed === '' ? null : $trimmed;
    }

    /**
     * Deletes a player and revokes their tokens, then forgets them. An
     * unknown player is a success too; on a failure the row is kept. Call it
     * from your account-deletion path.
     *
     * @throws \Lingara\Exception\LingaraException
     */
    public function deletePlayer(string $playerRef): void
    {
        $this->client->deleteEmbedPlayer($playerRef);
        $this->directory?->forget($playerRef);
    }

    /** @return array<string, mixed> */
    public function __debugInfo(): array
    {
        return ['client' => $this->client, 'directory' => $this->directory !== null];
    }

    /** @return array<string, mixed> */
    public function __serialize(): array
    {
        throw new \LogicException('an EmbedServer cannot be serialized');
    }

    /** @param array<mixed> $data */
    public function __unserialize(array $data): void
    {
        throw new \LogicException('an EmbedServer cannot be unserialized');
    }
}
