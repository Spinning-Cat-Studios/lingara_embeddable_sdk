package com.getlingara.embed.server.kotlin

import com.getlingara.kotlin.events.Webhook
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long
import java.io.File
import java.time.Clock
import java.time.Instant
import java.time.ZoneOffset
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class PlayerRouterTest {
    /** A directory that counts lookups, and fails them when asked to. */
    private class Counting(
        private val fail: Boolean,
    ) : PlayerDirectory {
        val inner = MemoryPlayerDirectory()
        var lookups = 0

        override suspend fun remember(
            playerRef: String,
            subject: String,
        ) = inner.remember(playerRef, subject)

        override suspend fun playerRefFor(subject: String): String? {
            lookups++
            check(!fail) { "the store is down" }
            return inner.playerRefFor(subject)
        }

        override suspend fun forget(playerRef: String) = inner.forget(playerRef)
    }

    /** One E7 signature vector: its verifier at the vector's own now, its body and headers. */
    private class Vector(
        val webhook: Webhook,
        val body: ByteArray,
        val headers: Map<String, List<String>>,
    )

    private fun vector(name: String): Vector {
        val all = Json.parseToJsonElement(File(System.getProperty("lingara.webhookVectors")).readText())
        val v =
            all.jsonObject
                .getValue("vectors")
                .jsonArray
                .map { it.jsonObject }
                .first { it.text("name") == name }
        val secrets = v.getValue("secrets").jsonArray.map { it.jsonPrimitive.content }
        val at = Clock.fixed(Instant.ofEpochSecond(v.getValue("now").jsonPrimitive.long), ZoneOffset.UTC)
        val headers = v.getValue("headers").jsonObject.mapValues { listOf(it.value.jsonPrimitive.content) }
        return Vector(Webhook(*secrets.toTypedArray(), clock = at), v.text("body").toByteArray(), headers)
    }

    private fun JsonObject.text(key: String): String = getValue(key).jsonPrimitive.content

    private suspend fun directory(fail: Boolean = false) = Counting(fail).also { it.inner.remember("p1", "lgr_sub_conformance") }

    /**
     * 1.10.26af AC18: through `handle`, a tampered body is 401 and never reaches the directory or
     * `onEvent`; a valid event reaches `onEvent` with the remembered player_ref; an unknown subject
     * arrives with none; a raising handler and a raising directory are each 500 and reach
     * `onError`; a valid signature over a body that is not an event is 400.
     */
    @Test
    fun handleVerifiesBeforeResolving() =
        runBlocking {
            val seen = mutableListOf<String?>()
            val record: suspend (RoutedEvent) -> Unit = { seen.add(it.playerRef) }

            val tampered = vector("tampered-body")
            val dir = directory()
            assertEquals(401, PlayerRouter(dir, tampered.webhook).handle(tampered.body, tampered.headers, record))
            assertEquals(0, dir.lookups)
            assertTrue(seen.isEmpty())

            val valid = vector("valid-single-signature")
            val router = PlayerRouter(dir, valid.webhook)
            assertEquals(204, router.handle(valid.body, valid.headers, record))
            assertEquals(1, dir.lookups)
            assertEquals(listOf<String?>("p1"), seen)

            dir.forget("p1")
            seen.clear()
            assertEquals(204, router.handle(valid.body, valid.headers, record))
            assertEquals(listOf<String?>(null), seen, "an unknown subject routes to no player")

            val errors = mutableListOf<String?>()
            val raising = PlayerRouter(directory(), valid.webhook) { errors.add(it.message) }
            assertEquals(500, raising.handle(valid.body, valid.headers) { error("the handler broke") })
            assertEquals(listOf<String?>("the handler broke"), errors)

            errors.clear()
            seen.clear()
            val broken = PlayerRouter(directory(fail = true), valid.webhook) { errors.add(it.message) }
            assertEquals(500, broken.handle(valid.body, valid.headers, record))
            assertEquals(listOf<String?>("the store is down"), errors)
            assertTrue(seen.isEmpty(), "a failed lookup never calls onEvent")

            val notJson = vector("body-not-json")
            assertEquals(400, PlayerRouter(directory(), notJson.webhook).handle(notJson.body, notJson.headers, record))
        }
}
