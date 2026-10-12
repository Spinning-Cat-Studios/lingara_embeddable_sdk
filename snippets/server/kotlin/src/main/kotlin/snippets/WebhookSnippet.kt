package snippets

// lingara:begin webhook
import com.getlingara.embed.server.kotlin.PlayerDirectory
import com.getlingara.embed.server.kotlin.PlayerRouter
import com.getlingara.kotlin.events.PracticeCompleted
import io.ktor.http.HttpStatusCode
import io.ktor.server.request.receive
import io.ktor.server.response.respond
import io.ktor.server.routing.Route
import io.ktor.server.routing.post
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import javax.sql.DataSource

// lingara:end
// lingara:begin webhook

/**
 * Your player directory, beside your player rows:
 *
 * ```sql
 * CREATE TABLE lingara_players (player_ref TEXT NOT NULL, subject TEXT NOT NULL);
 * CREATE UNIQUE INDEX ON lingara_players (subject);
 * CREATE INDEX ON lingara_players (player_ref);
 * ```
 */
class JdbcPlayerDirectory(
    private val db: DataSource,
) : PlayerDirectory {
    override suspend fun remember(
        playerRef: String,
        subject: String,
    ) = update(
        "INSERT INTO lingara_players (player_ref, subject) VALUES (?, ?) " +
            "ON CONFLICT (subject) DO UPDATE SET player_ref = excluded.player_ref",
        playerRef,
        subject,
    )

    override suspend fun playerRefFor(subject: String): String? =
        withContext(Dispatchers.IO) {
            db.connection.use { c ->
                c.prepareStatement("SELECT player_ref FROM lingara_players WHERE subject = ?").use { q ->
                    q.setString(1, subject)
                    q.executeQuery().use { row -> if (row.next()) row.getString(1) else null }
                }
            }
        }

    override suspend fun forget(playerRef: String) = update("DELETE FROM lingara_players WHERE player_ref = ?", playerRef)

    private suspend fun update(
        sql: String,
        vararg values: String,
    ) {
        withContext(Dispatchers.IO) {
            db.connection.use { c ->
                c.prepareStatement(sql).use { q ->
                    values.forEachIndexed { i, v -> q.setString(i + 1, v) }
                    q.executeUpdate()
                }
            }
        }
    }
}
// lingara:end

/** The documentation site's webhook example: the router, once, at startup. */
fun webhookRouter(db: DataSource): PlayerRouter {
    // lingara:begin webhook
    // Once, at startup: your webhook secret (lgr_whsec_…) from the environment.
    // A malformed secret fails here, not on the first delivery.
    val router = PlayerRouter(JdbcPlayerDirectory(db), System.getenv("LINGARA_WEBHOOK_SECRET"))
    // lingara:end
    return router
}

/** The documentation site's webhook example: the route. */
fun Route.webhookRoute(router: PlayerRouter) {
    // lingara:begin webhook
    // POST /lingara/webhook: hand handle() the raw bytes, before anything parses
    // them, and send the status it returns with an empty body.
    post("/lingara/webhook") {
        val headers =
            call.request.headers
                .entries()
                .associate { it.key to it.value }
        val status =
            router.handle(call.receive<ByteArray>(), headers) { routed ->
                // Delivery is at least once: skip an event.id you have seen.
                val event = routed.event
                val playerId = routed.playerRef
                if (event is PracticeCompleted && playerId != null) award(playerId, event)
            }
        call.respond(HttpStatusCode.fromValue(status))
    }
    // lingara:end
}

/** Your own game logic. */
@Suppress("UNUSED_PARAMETER")
private fun award(
    playerId: String,
    done: PracticeCompleted,
) = Unit
