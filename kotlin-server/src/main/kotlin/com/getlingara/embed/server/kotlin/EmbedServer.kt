package com.getlingara.embed.server.kotlin

import com.getlingara.kotlin.LingaraClient
import com.getlingara.kotlin.embed.MintedToken
import com.getlingara.kotlin.embed.createEmbedToken
import com.getlingara.kotlin.embed.deleteEmbedPlayer
import com.getlingara.kotlin.model.EmbedTokenRequest

/**
 * Mint and delete (ADR 1.10.26af D4, D9): one library call each, the [directory] written only
 * after a success. There is no cache: every [mintToken] is one `POST`, so a token [deletePlayer]
 * revoked on another instance is never handed out again.
 *
 * A failed call throws the library's own `LingaraException` unchanged; its token cache, retries
 * and deprecation hook are the library's. Built over a client with your client credentials.
 */
public class EmbedServer(
    private val client: LingaraClient,
    private val directory: PlayerDirectory? = null,
) {
    /**
     * Mints a token for one player: one `POST /v1/embed/tokens`, then `remember(playerRef,
     * subject)`. [scopes] and [origin] are sent only when given; [origin] is your page's origin, one
     * of your client's embed origins, and a game's server mints without one. On a failure nothing
     * is remembered.
     *
     * @throws IllegalArgumentException for a scope the library does not know
     */
    public suspend fun mintToken(
        playerRef: String,
        scopes: List<String>? = null,
        origin: String? = null,
    ): MintedToken {
        val request = EmbedTokenRequest(playerRef, scopes?.map(::scope)?.toSet(), origin)
        val minted = client.createEmbedToken(request).body
        directory?.remember(playerRef, minted.subject)
        return minted
    }

    /**
     * Deletes a player and revokes their tokens, then forgets them. An unknown player is a success;
     * on a failure the row is kept. Call it from your account-deletion path.
     */
    public suspend fun deletePlayer(playerRef: String) {
        client.deleteEmbedPlayer(playerRef)
        directory?.forget(playerRef)
    }

    override fun toString(): String = "EmbedServer(client=$client, directory=${directory != null})"

    private fun scope(name: String): EmbedTokenRequest.Scopes =
        EmbedTokenRequest.Scopes.entries.firstOrNull { it.value == name }
            ?: throw IllegalArgumentException("a scope this library does not know")
}
