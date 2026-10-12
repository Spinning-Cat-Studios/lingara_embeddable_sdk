package com.getlingara.embed.server.kotlin.conformance

import com.getlingara.embed.server.kotlin.PlayerRouter
import com.getlingara.embed.server.kotlin.RoutedEvent
import com.getlingara.kotlin.events.UnknownEvent
import io.ktor.http.HttpStatusCode
import io.ktor.server.application.ApplicationCall
import io.ktor.server.request.receive
import io.ktor.server.response.respond
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonPrimitive

/** What one delivery's `onEvent` saw. */
data class Seen(
    val playerRef: String?,
    val eventId: String,
    val unknown: Boolean,
)

/** The armed case, as the receiver needs it. */
class Armed(
    val deliver: List<JsonElement>,
    val directory: Recording,
) {
    val seen: MutableList<Seen?> = MutableList(deliver.size) { null }
    var received = 0
    var answered = 0
}

/**
 * The webhook receiver the conformance server posts to. Delivery `n` of the armed case is answered
 * through a [PlayerRouter] built from that item's `router.secrets`, whose `onEvent` records what
 * it saw and throws when the item's `handler` is `raise`.
 */
class Receiver(
    /** `primary` and `secondary`, from `LINGARA_EMBED_WEBHOOK_SECRETS`. */
    private val secrets: Map<String, String>,
    val sink: Sink,
) {
    private var armed: Armed? = null

    fun arm(next: Armed?) = synchronized(this) { armed = next }

    fun answered(count: Int): Boolean = synchronized(this) { armed?.let { it.answered >= count } ?: true }

    fun seen(): List<Seen?> = synchronized(this) { armed?.seen?.toList() ?: emptyList() }

    /** The router delivery [item] is answered through. */
    fun router(
        item: JsonElement,
        directory: Recording,
    ): PlayerRouter {
        val named = ((item as? JsonObject)?.get("router") as? JsonObject)?.get("secrets")?.jsonArray
        val names = named?.map { it.jsonPrimitive.content } ?: listOf("primary")
        val values = names.map { secrets[it] ?: throw IllegalArgumentException("no secret named $it") }
        return PlayerRouter(directory, *values.toTypedArray(), onError = { sink.push("on_error: $it") })
    }

    suspend fun deliver(call: ApplicationCall) {
        val body = call.receive<ByteArray>()
        val headers =
            call.request.headers
                .entries()
                .associate { it.key to it.value }
        val claimed =
            synchronized(this) {
                armed?.let { a -> Triple(a.received++, a.deliver.getOrElse(a.received - 1) { JsonNull }, a.directory) }
            }
        if (claimed == null) return call.respond(HttpStatusCode.Conflict)
        val (n, item, directory) = claimed
        val status = answer(n, item, directory, body, headers)
        synchronized(this) { armed?.let { it.answered++ } }
        call.respond(HttpStatusCode.fromValue(status))
    }

    private suspend fun answer(
        n: Int,
        item: JsonElement,
        directory: Recording,
        body: ByteArray,
        headers: Map<String, List<String>>,
    ): Int {
        val router =
            try {
                router(item, directory)
            } catch (e: IllegalArgumentException) {
                sink.push("harness: ${e.message}")
                return 500
            }
        val raise = (item as? JsonObject)?.get("handler")?.jsonPrimitive?.content == "raise"
        return router.handle(body, headers) { record(n, it, raise) }
    }

    private fun record(
        n: Int,
        routed: RoutedEvent,
        raise: Boolean,
    ) {
        val seen = Seen(routed.playerRef, routed.event.id, routed.event is UnknownEvent)
        synchronized(this) { armed?.seen?.let { if (n < it.size) it[n] = seen } }
        check(!raise) { "the conformance handler raised, as the case asked" }
    }
}
