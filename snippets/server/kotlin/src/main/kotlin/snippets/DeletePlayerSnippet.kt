package snippets

import com.getlingara.embed.server.kotlin.EmbedServer

/** The documentation site's delete example: your account-deletion path. */
suspend fun deleteAccount(
    embed: EmbedServer,
    playerId: String,
) {
    // lingara:begin deletePlayer
    // From your account-deletion path: revokes every token the player holds and
    // forgets their subjects. An unknown player is a success, so a retry is safe.
    embed.deletePlayer(playerId)
    // lingara:end
}
