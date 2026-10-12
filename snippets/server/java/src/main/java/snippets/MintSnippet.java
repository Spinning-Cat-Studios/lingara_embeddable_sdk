package snippets;

// lingara:begin mint
import com.getlingara.client.LingaraClient;
import com.getlingara.client.MintedToken;
import com.getlingara.embed.server.EmbedServer;
import com.getlingara.embed.server.MintOptions;
import com.getlingara.embed.server.PlayerDirectory;
import com.getlingara.embed.server.TokenHandoff;
import com.sun.net.httpserver.HttpExchange;
import java.io.IOException;
import java.nio.charset.StandardCharsets;

// lingara:end
/** The documentation site's mint example. */
public final class MintSnippet {
  private MintSnippet() {}

  /**
   * Builds the server once, at startup.
   *
   * @param players your database-backed directory
   * @return the server
   */
  public static EmbedServer embedServer(PlayerDirectory players) {
    // lingara:begin mint
    // Once, at startup. The credentials are your metered client's (lgr_ci_…,
    // lgr_cs_…), from the environment; never ship them to a device.
    LingaraClient client =
        LingaraClient.builder()
            .clientCredentials(
                System.getenv("LINGARA_CLIENT_ID"), System.getenv("LINGARA_CLIENT_SECRET"))
            .build();
    EmbedServer embed = EmbedServer.of(client, players);
    // lingara:end
    return embed;
  }

  /**
   * POST /lingara/token.
   *
   * @param embed the server
   * @param exchange the request
   * @throws IOException when the response cannot be written
   */
  public static void tokenRoute(EmbedServer embed, HttpExchange exchange) throws IOException {
    // lingara:begin mint
    // POST /lingara/token: your own route, behind your own login, rate-limited
    // per session. Every call is one mint; the widget calls it with
    // same-origin cookies.
    String playerId = currentPlayer(exchange);
    if (playerId == null) {
      exchange.sendResponseHeaders(401, -1);
      return;
    }
    MintedToken minted =
        embed.mintToken(playerId, MintOptions.none().withOrigin("https://game.example"));
    byte[] body = TokenHandoff.body(minted).getBytes(StandardCharsets.UTF_8);
    exchange.getResponseHeaders().set("Content-Type", "application/json");
    exchange.getResponseHeaders().set("Cache-Control", "no-store");
    exchange.sendResponseHeaders(200, body.length); // {"token":"lgr_et_…","expires_at":"…"}
    exchange.getResponseBody().write(body);
    exchange.close();
    // lingara:end
  }

  /** Your own session lookup. */
  private static String currentPlayer(HttpExchange exchange) {
    return exchange.getRequestHeaders().getFirst("X-Example-Player");
  }
}
