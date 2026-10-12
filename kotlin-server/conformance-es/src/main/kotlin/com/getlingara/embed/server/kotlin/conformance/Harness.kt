// The ES harness on the Kotlin server kit (ADR 1.10.26af D13): every ES case through
// com.getlingara.embed.server.kotlin and nothing else.
//
// `conformance-server run --tier es --lang kotlin-server -- <this>` starts it with the server's
// URL, the results path, the receiver's two webhook secrets and the visibly fake client
// credentials. It binds its webhook receiver (Ktor CIO) to port 0, prints `listening <port>`, and
// then for each case: fetches it, builds a fresh client, server and directory, arms it, runs the
// `kit:` jobs, waits for every delivery, judges what only the caller sees, finishes, and writes
// one result line. There is no skip.

package com.getlingara.embed.server.kotlin.conformance

import com.getlingara.embed.server.kotlin.EmbedServer
import com.getlingara.kotlin.LingaraClient
import io.ktor.server.cio.CIO
import io.ktor.server.engine.embeddedServer
import io.ktor.server.routing.route
import io.ktor.server.routing.routing
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
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
import java.io.File
import java.io.IOException
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import kotlin.system.exitProcess

/** The language this harness reports: make/kotlin-server.mk's ES_LANGS entry. */
private const val LANG = "kotlin-server"
private val SECRET_PREFIXES = listOf("lgr_et_", "lgr_cs_", "lgr_whsec_")
private const val WAIT_MILLIS = 15_000L

private fun env(name: String): String = System.getenv(name) ?: error("$name is not set")

fun main(): Unit =
    runBlocking {
        val sink = Sink().also { it.captureLogs() }
        val named = env("LINGARA_EMBED_WEBHOOK_SECRETS").split(",")
        val secrets = listOf("primary", "secondary").zip(named).toMap()
        val receiver = Receiver(secrets, sink)
        val server =
            embeddedServer(CIO, port = 0, host = "127.0.0.1") {
                routing { route("{...}") { handle { receiver.deliver(call) } } }
            }.start(wait = false)
        println("listening ${server.engine.resolvedConnectors().first().port}")
        System.out.flush()

        val harness = Harness(receiver)
        val cases = File(System.getenv("LINGARA_EMBED_CASES") ?: "conformance/es/cases")
        File(env("LINGARA_EMBED_RESULTS")).bufferedWriter().use { out ->
            for (id in caseIds(cases)) {
                out.write(harness.runCase(id).toString())
                out.newLine()
                out.flush()
            }
        }
        server.stop(0, 0)
        exitProcess(0)
    }

/** `<group>/<slug>.yaml` to `<group>.<slug>`, sorted. */
private fun caseIds(dir: File): List<String> =
    dir
        .listFiles()
        .orEmpty()
        .filter { it.isDirectory }
        .flatMap { group ->
            group
                .listFiles()
                .orEmpty()
                .filter { it.extension == "yaml" }
                .map { "${group.name}.${it.nameWithoutExtension}" }
        }.sorted()

private class Harness(
    private val receiver: Receiver,
) {
    private val http = HttpClient.newHttpClient()
    private val base = env("LINGARA_EMBED_SERVER_URL")

    suspend fun runCase(id: String): JsonElement {
        val case =
            try {
                Json.parseToJsonElement(call("GET", control("cases", id), null)).jsonObject
            } catch (e: IOException) {
                return result(id, listOf("cannot read the case: ${e.message}"))
            }
        val directory = Recording()
        val seed = ((case["directory"] as? JsonObject)?.get("seed") as? JsonObject).orEmpty()
        seed.forEach { (subject, player) -> directory.remember(player.jsonPrimitive.content, subject) }
        val deliver = (case["deliver"] as? JsonArray).orEmpty()
        receiver.arm(Armed(deliver, directory))
        receiver.sink.take()
        val embed = EmbedServer(client(), directory)

        val mismatches = mutableListOf<String>()
        try {
            call("POST", control("arm", id), "")
        } catch (e: IOException) {
            mismatches.add("cannot arm: ${e.message}")
        }
        val kit = Kit().also { it.run(embed, (case["kit"] as? JsonArray).orEmpty()) }
        val seen = waitForDeliveries(deliver.size)
        mismatches += kit.judge(id, (case["expect"] as? JsonObject) ?: JsonObject(emptyMap()), directory)
        mismatches += handlerMismatches(id, deliver, seen)
        if (kit.render) mismatches += renderMismatches(id, embed, directory, kit)
        mismatches += finish(id, seen)
        receiver.arm(null)
        return result(id, mismatches)
    }

    private fun client(): LingaraClient =
        LingaraClient {
            clientCredentials(env("LINGARA_EMBED_CLIENT_ID"), env("LINGARA_EMBED_CLIENT_SECRET"))
            baseUrl = URI.create(base)
            tokenUrl = URI.create("$base/oauth/token")
        }

    private suspend fun finish(
        id: String,
        seen: List<Seen?>,
    ): List<String> {
        val body = buildJsonObject { putJsonArray("handlers") { seen.forEach { add(JsonPrimitive(it?.playerRef)) } } }
        return try {
            val answer = Json.parseToJsonElement(call("POST", control("finish", id), body.toString())).jsonObject
            (answer["mismatches"] as? JsonArray).orEmpty().map { it.jsonPrimitive.content }
        } catch (e: IOException) {
            listOf("cannot finish: ${e.message}")
        }
    }

    private fun control(
        verb: String,
        id: String,
    ) = "$base/__conformance/$verb/$id"

    private suspend fun call(
        method: String,
        url: String,
        json: String?,
    ): String {
        val publisher = json?.let(HttpRequest.BodyPublishers::ofString) ?: HttpRequest.BodyPublishers.noBody()
        val request =
            HttpRequest
                .newBuilder(URI.create(url))
                .method(method, publisher)
                .header("content-type", "application/json")
                .build()
        val response = withContext(Dispatchers.IO) { http.send(request, HttpResponse.BodyHandlers.ofString()) }
        if (response.statusCode() >= 400) throw IOException("$method $url answered ${response.statusCode()}")
        return response.body()
    }

    /** Until every delivery has been answered, or 15 s; then what each saw. */
    private suspend fun waitForDeliveries(count: Int): List<Seen?> {
        val deadline = System.currentTimeMillis() + WAIT_MILLIS
        while (!receiver.answered(count) && System.currentTimeMillis() < deadline) delay(20)
        return receiver.seen()
    }

    /** ES5: no rendering of a kit object, and no captured log line, carries a secret prefix. */
    private fun renderMismatches(
        id: String,
        embed: EmbedServer,
        directory: Recording,
        kit: Kit,
    ): List<String> {
        val both =
            buildJsonObject {
                put(
                    "router",
                    buildJsonObject {
                        putJsonArray("secrets") {
                            add(JsonPrimitive("primary"))
                            add(JsonPrimitive("secondary"))
                        }
                    },
                )
            }
        val renderings =
            listOf(embed.toString(), receiver.router(both, directory).toString(), directory.toString(), kit.minted.toString()) +
                listOfNotNull(kit.minted?.token?.toString()) + receiver.sink.take()
        val leaks = renderings.count { r -> SECRET_PREFIXES.any { it in r } }
        return if (leaks == 0) emptyList() else listOf("$id: fail render: $leaks rendering(s) carry a secret prefix")
    }
}

/** Each delivery's `expect.handler` against what its `onEvent` saw. */
private fun handlerMismatches(
    id: String,
    deliver: List<JsonElement>,
    seen: List<Seen?>,
): List<String> =
    deliver.indices.mapNotNull { n ->
        val want = ((deliver[n] as? JsonObject)?.get("expect") as? JsonObject)?.get("handler") ?: JsonNull
        val got = seen.getOrNull(n)
        if (handlerMatches(want, got)) null else "$id: fail handler: deliver[$n] saw $got, expected $want"
    }

private fun handlerMatches(
    want: JsonElement,
    got: Seen?,
): Boolean {
    if (want is JsonPrimitive && want.isString) return want.content == "not_called" && got == null
    if (want !is JsonObject || got == null) return false
    return same(want, "player_ref", got.playerRef) &&
        same(want, "event_id", got.eventId) &&
        (want["unknown"] as? JsonPrimitive)?.let { it.content.toBoolean() == got.unknown } ?: true
}

/** A case value against an observed one, when the case names it: JSON null is "none". */
private fun same(
    want: JsonObject,
    key: String,
    got: String?,
): Boolean {
    val value = want[key] ?: return true
    return if (value is JsonNull) got == null else value.jsonPrimitive.content == got
}

private fun result(
    id: String,
    mismatches: List<String>,
): JsonElement =
    buildJsonObject {
        put("case", id)
        put("lang", LANG)
        put("result", if (mismatches.isEmpty()) "pass" else "fail")
        putJsonArray("mismatches") { mismatches.forEach { add(JsonPrimitive(it)) } }
    }
