<?php

declare(strict_types=1);

namespace Lingara\Embed;

use Lingara\Client;
use Lingara\MintedToken;
use Lingara\Model\EmbedTokenRequest;

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
     * `remember($playerRef, $minted->subject)`. `scopes` and `origin` are
     * sent only when given; `origin` is your page's origin, one of your
     * client's embed origins (a game's server mints without one). On a
     * failure nothing is remembered.
     *
     * @param list<string>|null $scopes
     *
     * @throws \Lingara\Exception\LingaraException
     * @throws \InvalidArgumentException for a value the request model refuses before anything is sent
     */
    public function mintToken(string $playerRef, ?array $scopes = null, ?string $origin = null): MintedToken
    {
        // `origin` goes in through the constructor, not setOrigin(): the
        // library's 0.2.0-alpha.1 setter calls an ObjectSerializer::toString()
        // that does not exist, so it throws \Error for any non-null origin.
        // Lingara checks the origin's shape and registration either way (B1).
        $request = (new EmbedTokenRequest($origin === null ? null : ['origin' => $origin]))->setPlayerRef($playerRef);
        if ($scopes !== null) {
            $request->setScopes($scopes);
        }
        $minted = $this->client->createEmbedToken($request)->value;
        $this->directory?->remember($playerRef, $minted->subject);
        return $minted;
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
