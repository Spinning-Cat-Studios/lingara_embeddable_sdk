/**
 * The Lingara embed server kit for Java (ADR 1.10.26af D2): mint a player's token, hand it over,
 * route a verified webhook to its player, delete a player, and render the widget snippet.
 *
 * <p>The client, its errors, {@code MintedToken} and the webhook verifier are the Lingara
 * library's, so the library is the one module the kit requires, transitively: every signature here
 * names its types.
 */
module com.getlingara.embed.server {
  requires transitive com.getlingara.client;

  exports com.getlingara.embed.server;
}
