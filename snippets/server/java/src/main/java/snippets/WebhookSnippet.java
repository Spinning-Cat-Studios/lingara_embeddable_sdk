package snippets;

// lingara:begin webhook
import com.getlingara.client.events.PracticeCompleted;
import com.getlingara.embed.server.PlayerDirectory;
import com.getlingara.embed.server.PlayerRouter;
import com.sun.net.httpserver.HttpExchange;
import java.io.IOException;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.Optional;
import javax.sql.DataSource;

// lingara:end
/** The documentation site's webhook example. */
public final class WebhookSnippet {
  private WebhookSnippet() {}

  // lingara:begin webhook
  /**
   * Your player directory, beside your player rows:
   *
   * <pre>
   * CREATE TABLE lingara_players (player_ref TEXT NOT NULL, subject TEXT NOT NULL);
   * CREATE UNIQUE INDEX ON lingara_players (subject);
   * CREATE INDEX ON lingara_players (player_ref);
   * </pre>
   */
  public record JdbcPlayerDirectory(DataSource db) implements PlayerDirectory {
    @Override
    public void remember(String playerRef, String subject) {
      update(
          "INSERT INTO lingara_players (player_ref, subject) VALUES (?, ?)"
              + " ON CONFLICT (subject) DO UPDATE SET player_ref = excluded.player_ref",
          playerRef,
          subject);
    }

    @Override
    public Optional<String> playerRefFor(String subject) {
      try (Connection c = db.getConnection();
          PreparedStatement q =
              c.prepareStatement("SELECT player_ref FROM lingara_players WHERE subject = ?")) {
        q.setString(1, subject);
        try (ResultSet row = q.executeQuery()) {
          return row.next() ? Optional.of(row.getString(1)) : Optional.empty();
        }
      } catch (SQLException e) {
        throw new IllegalStateException("the player directory failed", e);
      }
    }

    @Override
    public void forget(String playerRef) {
      update("DELETE FROM lingara_players WHERE player_ref = ?", playerRef);
    }

    private void update(String sql, String... values) {
      try (Connection c = db.getConnection();
          PreparedStatement q = c.prepareStatement(sql)) {
        for (int i = 0; i < values.length; i++) {
          q.setString(i + 1, values[i]);
        }
        q.executeUpdate();
      } catch (SQLException e) {
        throw new IllegalStateException("the player directory failed", e);
      }
    }
  }

  // lingara:end
  /**
   * Builds the router once, at startup.
   *
   * @param db your database
   * @return the router
   */
  public static PlayerRouter webhookRouter(DataSource db) {
    // lingara:begin webhook
    // Once, at startup: your webhook secret (lgr_whsec_…) from the environment.
    // A malformed secret fails here, not on the first delivery.
    PlayerRouter router =
        PlayerRouter.of(new JdbcPlayerDirectory(db), System.getenv("LINGARA_WEBHOOK_SECRET"));
    // lingara:end
    return router;
  }

  /**
   * POST /lingara/webhook.
   *
   * @param router the router
   * @param exchange the request
   * @throws IOException when the response cannot be written
   */
  public static void webhookRoute(PlayerRouter router, HttpExchange exchange) throws IOException {
    // lingara:begin webhook
    // POST /lingara/webhook: hand handle() the raw bytes, before anything parses
    // them, and send the status it returns with an empty body.
    byte[] body = exchange.getRequestBody().readAllBytes();
    int status =
        router.handle(
            body,
            exchange.getRequestHeaders(),
            routed -> {
              // Delivery is at least once: skip an event.id you have seen.
              if (routed.event() instanceof PracticeCompleted done
                  && routed.playerRef().isPresent()) {
                award(routed.playerRef().get(), done);
              }
            });
    exchange.sendResponseHeaders(status, -1);
    exchange.close();
    // lingara:end
  }

  /** Your own game logic. */
  private static void award(String playerId, PracticeCompleted done) {
    // Credit the player for done.data().
  }
}
