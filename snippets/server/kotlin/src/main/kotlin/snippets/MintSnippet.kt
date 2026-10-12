package snippets

// lingara:begin mint
import com.getlingara.embed.server.kotlin.EmbedServer
import com.getlingara.embed.server.kotlin.PlayerDirectory
import com.getlingara.embed.server.kotlin.tokenHandoff
import com.getlingara.kotlin.LingaraClient
import io.ktor.http.ContentType
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpStatusCode
import io.ktor.server.response.header
import io.ktor.server.response.respond
import io.ktor.server.response.respondText
import io.ktor.server.routing.Route
import io.ktor.server.routing.post

// lingara:end

/** The documentation site's mint example: the server, once, at startup. */
fun embedServer(players: PlayerDirectory): EmbedServer {
    // lingara:begin mint
    // Once, at startup. The credentials are your metered client's (lgr_ci_…,
    // lgr_cs_…), from the environment; never ship them to a device.
    val client =
        LingaraClient {
            clientCredentials(System.getenv("LINGARA_CLIENT_ID"), System.getenv("LINGARA_CLIENT_SECRET"))
        }
    val embed = EmbedServer(client, players)
    // lingara:end
    return embed
}

/** The documentation site's mint example: the token route. */
fun Route.tokenRoute(embed: EmbedServer) {
    // lingara:begin mint
    // POST /lingara/token: your own route, behind your own login, rate-limited
    // per session. Every call is one mint; the widget calls it with
    // same-origin cookies.
    post("/lingara/token") {
        val playerId = call.request.headers["X-Example-Player"] ?: return@post call.respond(HttpStatusCode.Unauthorized)
        val minted = embed.mintToken(playerId, origin = "https://game.example")
        call.response.header(HttpHeaders.CacheControl, "no-store")
        call.respondText(tokenHandoff(minted), ContentType.Application.Json) // {"token":"lgr_et_…","expires_at":"…"}
    }
    // lingara:end
}
