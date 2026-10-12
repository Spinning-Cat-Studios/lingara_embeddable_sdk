package com.getlingara.embed.server.kotlin.conformance

import com.getlingara.embed.server.kotlin.EmbedServer
import com.getlingara.embed.server.kotlin.tokenHandoff
import com.getlingara.kotlin.ApiException
import com.getlingara.kotlin.MaintenanceException
import com.getlingara.kotlin.OAuthException
import com.getlingara.kotlin.TransportException
import com.getlingara.kotlin.embed.MintedToken
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonArray
import kotlin.coroutines.cancellation.CancellationException

/**
 * A case's `kit:` jobs through [EmbedServer], and the comparisons only the kit's caller can make
 * (ES `CONTRACT.md`, What a harness reports).
 */
class Kit {
    var minted: MintedToken? = null
    var handoff: String? = null
    var error: JsonElement? = null
    var render = false

    /** Every job in order; a raised error is recorded and the next job runs. */
    suspend fun run(
        server: EmbedServer,
        jobs: List<JsonElement>,
    ) {
        for (job in jobs.map { it.jsonObject }) {
            try {
                step(server, job)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                error = raised(e)
            }
        }
    }

    private suspend fun step(
        server: EmbedServer,
        job: JsonObject,
    ) {
        val mint = job["mint"]?.jsonObject
        val delete = job["delete"]?.jsonObject
        when {
            mint != null -> minted = server.mintToken(mint.text("player_ref")!!, scopes(mint), mint.text("origin"))
            delete != null -> server.deletePlayer(delete.text("player_ref")!!)
            "handoff" in job -> handoff = minted?.let(::tokenHandoff)
            else -> render = true
        }
    }

    private fun scopes(m: JsonObject): List<String>? = (m["scopes"] as? JsonArray)?.map { it.jsonPrimitive.content }

    /** `{status, code}`: an API error's, an OAuth error's `error`, maintenance. */
    private fun raised(e: Exception): JsonElement {
        val (status, code) =
            when (e) {
                is ApiException -> e.status to e.code
                is OAuthException -> e.status to e.error
                is MaintenanceException -> 503 to "maintenance"
                is TransportException -> 0 to e.kind.wireName
                else -> 0 to (e::class.simpleName ?: "error")
            }
        return buildJsonObject {
            put("status", status)
            put("code", code)
        }
    }

    /** The six fields in wire form, the token read through its accessor. */
    private fun wire(t: MintedToken): JsonElement =
        buildJsonObject {
            put("token", t.token.exposeSecret())
            put("expires_at", t.expiresAt)
            put("expires_in", t.expiresIn.seconds)
            put("subject", t.subject)
            putJsonArray("scopes") { t.scopes.forEach { add(JsonPrimitive(it)) } }
            put("account_linked", t.accountLinked)
        }

    private fun got(
        key: String,
        directory: Recording,
    ): JsonElement? =
        when (key) {
            "minted" -> minted?.let(::wire)
            "handoff" -> handoff?.let(::JsonPrimitive)
            "error" -> error
            "directory" -> JsonObject(directory.snapshot().mapValues { JsonPrimitive(it.value) })
            else -> JsonPrimitive(directory.lookups())
        }

    /** Each compared key, a `fail <key>` line when it differs. */
    fun judge(
        id: String,
        expect: JsonObject,
        directory: Recording,
    ): List<String> =
        listOf("minted", "handoff", "error", "directory", "directory_lookups").mapNotNull { key ->
            val want = expect[key]?.let(::withoutNulls)
            val have = got(key, directory)?.let(::withoutNulls)
            // A key the case names must match; an error it did not expect is a mismatch too.
            val differs = if (want != null) want != have else key == "error" && have != null
            if (differs) "$id: fail $key: got ${have ?: "nothing"}, expected ${want ?: "nothing"}" else null
        }
}

/** [value] with every null object member dropped, at any depth. */
fun withoutNulls(value: JsonElement): JsonElement =
    when (value) {
        is JsonObject -> JsonObject(value.filterValues { it !is JsonNull }.mapValues { withoutNulls(it.value) })
        is JsonArray -> JsonArray(value.map(::withoutNulls))
        is JsonPrimitive -> value
    }

private fun JsonObject.text(key: String): String? = (this[key] as? JsonPrimitive)?.takeIf { it !is JsonNull }?.content
