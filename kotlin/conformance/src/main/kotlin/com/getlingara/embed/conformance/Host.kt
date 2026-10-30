// A case's `host:` script on the Kotlin surface (ADR 1.10.26ae D9). Each step
// goes through LingaraEmbed's public methods only. A command is read with
// commandFromJson and every event written with eventToJson, so this harness
// never spells K1's JSON shape itself. A step the core refuses becomes an
// `error` event in the harness's view, seen before anything still queued.

package com.getlingara.embed.conformance

import com.getlingara.embed.Command
import com.getlingara.embed.CommandOutcome
import com.getlingara.embed.EmbedConfig
import com.getlingara.embed.EmbedEvent
import com.getlingara.embed.EmbedException
import com.getlingara.embed.LingaraEmbed
import com.getlingara.embed.LogLevel
import com.getlingara.embed.Secret
import com.getlingara.embed.commandFromJson
import com.getlingara.embed.eventToJson
import com.getlingara.embed.provideToken
import com.getlingara.embed.provideTokenFor
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull

/** How long `close` waits for the one `Closed`. */
private const val CLOSE_WITHIN_MS = 2_000L

internal class Host(
    private val server: String,
) {
    val problems = mutableListOf<String>()
    val matched = mutableListOf<JsonElement>()
    private var embed: LingaraEmbed? = null
    private val local = ArrayDeque<JsonElement>()
    private var lastId: Long? = null

    fun run(case: JsonObject): Host {
        val id = case["id"]?.jsonPrimitive?.content.orEmpty()
        for ((n, step) in case["host"]?.jsonArray.orEmpty().withIndex()) {
            val (name, body) = step.jsonObject.entries.firstOrNull() ?: continue
            val problem = runCatching { step(name, body) }.exceptionOrNull()
            if (problem != null) {
                problems += "$id: host[$n].$name: ${problem.message}"
                break
            }
        }
        embed?.close()
        return this
    }

    private fun embed(): LingaraEmbed = embed ?: error("no start step ran")

    private fun step(
        name: String,
        body: JsonElement,
    ) {
        when (name) {
            "start" -> start(body.jsonObject)
            "provide_token" -> provide(body.jsonObject)
            "command" -> command(body)
            "poll_until" -> pollUntil(body.jsonObject)
            "expect_no_event" -> expectNoEvent(body.jsonObject)
            "stop_polling" -> Thread.sleep(body.jsonObject.long("for_ms") ?: 0L)
            "close" -> close()
            else -> error("unknown step $name")
        }
    }

    private fun start(body: JsonObject) {
        val c = body["config"]?.jsonObject ?: JsonObject(emptyMap())
        val config =
            EmbedConfig(
                baseUrl = server,
                apiVersion = c.string("api_version"),
                eventTypes = c["event_types"]?.jsonArray?.map { it.jsonPrimitive.content }.orEmpty(),
                resumeCursor = c.string("resume_cursor"),
                streamOnStart = c["stream_on_start"]?.jsonPrimitive?.booleanOrNull,
                queueCapacity = c.long("queue_capacity"),
                streamIdleTimeoutMs = c.long("stream_idle_timeout_ms"),
                logLevel = c.string("log_level")?.let { LogLevel.valueOf(it.uppercase()) },
                userAgentSuffix = c.string("user_agent_suffix"),
            )
        val started = LingaraEmbed.start(config)
        embed = started
        body.string("initial_token")?.takeIf { it != "none" }?.let { started.provideTokenFor(Secret(it), 900) }
    }

    private fun provide(body: JsonObject) {
        val token = Secret(body.string("token").orEmpty())
        val at = body.string("expires_at")
        val secs = body.long("expires_in_secs")
        refusing {
            when {
                at != null -> embed().provideToken(token, at)
                secs != null -> embed().provideTokenFor(token, secs)
                else -> error("no expires_at or expires_in_secs")
            }
        }
    }

    /** A Command in its JSON shape; any value `"$last"` is the latest opened session or dialogue id. */
    private fun command(body: JsonElement) {
        val command = commandFromJson(substitute(body).toString())
        val opens = command is Command.PracticeStart || command is Command.DialogueOpen || command is Command.DialogueLoad
        refusing {
            val outcome = embed().execute(command)
            if (opens && outcome is CommandOutcome.Id) lastId = outcome.id
        }
    }

    private fun refusing(call: () -> Unit) {
        try {
            call()
        } catch (e: EmbedException.Failed) {
            local.addLast(Json.parseToJsonElement(eventToJson(EmbedEvent.Error(e.error))))
        }
    }

    private fun substitute(value: JsonElement): JsonElement =
        when {
            value is JsonPrimitive && value.isString && value.content == "\$last" -> JsonPrimitive(lastId)
            value is JsonObject -> JsonObject(value.mapValues { substitute(it.value) })
            value is JsonArray -> JsonArray(value.map(::substitute))
            else -> value
        }

    /** The next event, a refusal first. */
    private fun next(): JsonElement? = local.removeFirstOrNull() ?: embed?.poll()?.let { Json.parseToJsonElement(eventToJson(it)) }

    /** Drains until an event matches or the deadline passes. */
    private fun find(
        event: String,
        wanted: JsonElement?,
        deadline: Long,
    ): JsonElement? {
        while (System.currentTimeMillis() < deadline) {
            val seen = next()
            when {
                seen == null -> Thread.sleep(5)
                seen.jsonObject.string("type") == event && contains(seen, wanted) -> return seen
            }
        }
        return null
    }

    private fun pollUntil(body: JsonObject) {
        val event = body.string("event").orEmpty()
        val within = body.long("within_ms") ?: 0L
        val seen =
            find(event, body["match"], System.currentTimeMillis() + within)
                ?: error("no $event event matching ${body["match"]} within $within ms")
        matched += seen
    }

    private fun expectNoEvent(body: JsonObject) {
        val event = body.string("event").orEmpty()
        val seen = find(event, body["match"], System.currentTimeMillis() + (body.long("for_ms") ?: 0L))
        check(seen == null) { "an unexpected $event event: $seen" }
    }

    private fun close() {
        embed().shutdown()
        find("closed", null, System.currentTimeMillis() + CLOSE_WITHIN_MS) ?: error("no closed event after close")
        val after = embed().poll()
        check(after == null) { "an event after closed: $after" }
    }
}

private fun JsonObject.string(key: String): String? = (this[key] as? JsonPrimitive)?.contentOrNull

private fun JsonObject.long(key: String): Long? = (this[key] as? JsonPrimitive)?.longOrNull
