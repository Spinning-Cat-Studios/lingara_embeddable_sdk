// The EK harness on the Kotlin surface (ADR 1.10.26ae D9). conformance-server
// run starts it with LINGARA_EMBED_SERVER_URL and LINGARA_EMBED_RESULTS. For
// each case under conformance/ek/cases/ it fetches the case, arms it, runs
// its host script through LingaraEmbed, finishes it, and writes one result
// line merging the server's verdict with its own. It reports every case:
// there is no skip.

package com.getlingara.embed.conformance

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.put
import java.io.File
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse

/** The language this harness reports: make/kotlin.mk's EK_LANGS entry. */
private const val LANG = "kotlin"

private val http: HttpClient = HttpClient.newHttpClient()

fun main() {
    val server = System.getenv("LINGARA_EMBED_SERVER_URL") ?: error("LINGARA_EMBED_SERVER_URL")
    val results = System.getenv("LINGARA_EMBED_RESULTS") ?: error("LINGARA_EMBED_RESULTS")
    val cases = File(System.getenv("LINGARA_EMBED_CASES") ?: "conformance/ek/cases")
    // A failing case is in the results, which the server judges; the exit
    // code says only that every case was run and reported.
    File(results).printWriter().use { out ->
        for (id in caseIds(cases)) out.println(runCase(server, id))
    }
}

/** `<group>/<slug>.yaml` → `<group>.<slug>`, sorted. */
internal fun caseIds(dir: File): List<String> =
    dir
        .listFiles { f -> f.isDirectory }
        .orEmpty()
        .flatMap { group -> group.listFiles { f -> f.extension == "yaml" }.orEmpty().map { "${group.name}.${it.nameWithoutExtension}" } }
        .sorted()

private fun request(
    url: String,
    body: String?,
): String {
    val builder = HttpRequest.newBuilder(URI.create(url))
    val req =
        if (body ==
            null
        ) {
            builder.GET().build()
        } else {
            builder.header("content-type", "application/json").POST(HttpRequest.BodyPublishers.ofString(body)).build()
        }
    val response = http.send(req, HttpResponse.BodyHandlers.ofString())
    check(response.statusCode() in 200..299) { "$url answered ${response.statusCode()}" }
    return response.body()
}

/** One case end to end. Any failure to reach the server is a mismatch, so the case is still reported. */
private fun runCase(
    server: String,
    id: String,
): String {
    val control = { verb: String -> "$server/__conformance/$verb/$id" }
    val case =
        runCatching { Json.parseToJsonElement(request(control("cases"), null)).jsonObject }
            .getOrElse { return result(id, listOf("cannot read the case: ${it.message}")) }
    val mismatches = mutableListOf<String>()
    runCatching { request(control("arm"), "{}") }.onFailure { mismatches += "cannot arm: ${it.message}" }
    val run = Host(server).run(case)
    mismatches += run.problems
    case["expect"]?.jsonObject?.get("events")?.let { want ->
        val got = withoutNulls(JsonArray(run.matched))
        if (got != withoutNulls(want)) mismatches += "$id: the host saw $got, expected ${withoutNulls(want)}"
    }
    runCatching { Json.parseToJsonElement(request(control("finish"), "{}")).jsonObject }
        .onSuccess { verdict -> verdict["mismatches"]?.jsonArray?.forEach { mismatches += text(it) } }
        .onFailure { mismatches += "cannot finish: ${it.message}" }
    return result(id, mismatches)
}

private fun text(element: JsonElement): String = (element as? JsonPrimitive)?.content ?: element.toString()

private fun result(
    id: String,
    mismatches: List<String>,
): String =
    buildJsonObject {
        put("case", id)
        put("lang", LANG)
        put("result", if (mismatches.isEmpty()) "pass" else "fail")
        put("mismatches", JsonArray(mismatches.map(::JsonPrimitive)))
    }.toString()

/** An event matches when it contains the case's `match`, compared after dropping nulls. */
internal fun contains(
    actual: JsonElement,
    wanted: JsonElement?,
): Boolean =
    when {
        wanted == null || wanted is kotlinx.serialization.json.JsonNull -> true
        actual is JsonObject && wanted is JsonObject ->
            wanted.all { (k, v) -> v is kotlinx.serialization.json.JsonNull || actual[k]?.let { contains(it, v) } == true }
        actual is JsonArray && wanted is JsonArray -> actual.size == wanted.size && actual.zip(wanted).all { (a, w) -> contains(a, w) }
        else -> actual == wanted
    }

internal fun withoutNulls(value: JsonElement): JsonElement =
    when (value) {
        is JsonObject -> JsonObject(value.filterValues { it !is kotlinx.serialization.json.JsonNull }.mapValues { withoutNulls(it.value) })
        is JsonArray -> JsonArray(value.map(::withoutNulls))
        else -> value
    }
