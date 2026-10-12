package com.getlingara.embed

import java.io.File
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse

/**
 * `conformance-server serve` over the EK cases (ADR 1.10.26x D5), built by
 * `make test-kotlin` into target/debug: a test arms one case, drives the kit
 * against it, and reads the server's verdict at finish.
 */
internal class ConformanceServe : AutoCloseable {
    private val root = File(System.getProperty("lingara.root"))
    private val http = HttpClient.newHttpClient()
    private val process =
        ProcessBuilder(
            File(root, "target/debug/conformance-server").path,
            "serve",
            "--cases",
            File(root, "conformance/ek/cases").path,
            "--registry",
            File(root, "spec/versions.toml").path,
            "--port",
            "0",
        ).redirectError(ProcessBuilder.Redirect.INHERIT).start()

    val url: String

    init {
        val reader = process.inputStream.bufferedReader()
        val port =
            generateSequence { reader.readLine() }
                .firstNotNullOfOrNull { line -> line.removePrefix("listening ").takeIf { it != line } }
                ?: error("conformance-server printed no `listening <port>` line")
        url = "http://127.0.0.1:$port"
    }

    fun arm(case: String) {
        post("arm", case)
    }

    /** The server's verdict JSON for the case. */
    fun finish(case: String): String = post("finish", case)

    private fun post(
        verb: String,
        case: String,
    ): String {
        val request =
            HttpRequest
                .newBuilder(URI.create("$url/__conformance/$verb/$case"))
                .header("content-type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString("{}"))
                .build()
        val response = http.send(request, HttpResponse.BodyHandlers.ofString())
        check(response.statusCode() in 200..299) { "$verb $case answered ${response.statusCode()}: ${response.body()}" }
        return response.body()
    }

    override fun close() {
        process.destroy()
    }
}
