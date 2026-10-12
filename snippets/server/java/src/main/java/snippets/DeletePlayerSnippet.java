package snippets;

import com.getlingara.embed.server.EmbedServer;

/** The documentation site's delete example. */
public final class DeletePlayerSnippet {
  private DeletePlayerSnippet() {}

  /**
   * Your account-deletion path.
   *
   * @param embed the server
   * @param playerId your player's id
   */
  public static void deleteAccount(EmbedServer embed, String playerId) {
    // lingara:begin deletePlayer
    // From your account-deletion path: revokes every token the player holds and
    // forgets their subjects. An unknown player is a success, so a retry is safe.
    embed.deletePlayer(playerId);
    // lingara:end
  }
}
