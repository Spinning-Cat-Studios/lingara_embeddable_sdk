package com.getlingara.embed.server.kotlin

import com.getlingara.kotlin.model.UpdateAvailable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class VectorsTest {
    /**
     * 1.10.26af AC10: every `snippet.json` vector through [widgetSnippet], and every
     * `token-handoff.json` vector through the internal [handoffBody], gives its expected answer,
     * with `{{version}}` read as this kit's VERSION.
     *
     * 31.10.26f AC4: a handoff vector's `update_available` is passed as the library's
     * `UpdateAvailable`, and the body is byte-equal to the vector's.
     */
    @Test
    fun everySnippetAndHandoffVector() {
        val version = System.getProperty("lingara.version")
        val cases = read("lingara.snippetVectors").jsonObject.getValue("cases").jsonArray
        assertTrue(cases.isNotEmpty(), "snippet.json holds cases")
        for (case in cases) {
            val name = case.jsonObject.text("name")
            val options = options(case.jsonObject.getValue("options").jsonObject)
            val expect = case.jsonObject.getValue("expect").jsonObject
            val html = expect["html"]?.jsonPrimitive?.content
            if (html != null) {
                assertEquals(html.replace("{{version}}", version), widgetSnippet(options), name)
            } else {
                val e = assertFailsWith<SnippetException>(name) { widgetSnippet(options) }
                assertEquals(expect.text("refused"), e.field, name)
            }
        }

        val handoffs = read("lingara.handoffVectors").jsonArray
        assertTrue(handoffs.isNotEmpty(), "token-handoff.json holds cases")
        for (v in handoffs) {
            val input = v.jsonObject.getValue("input").jsonObject
            val body = handoffBody(input.text("token"), input.text("expires_at"), notice(input))
            assertEquals(v.jsonObject.text("body"), body, v.jsonObject.text("name"))
        }
    }

    /** The vector's `update_available`, when present, as the library's typed value. */
    private fun notice(input: JsonObject): UpdateAvailable? =
        input["update_available"]?.jsonObject?.let {
            UpdateAvailable(
                latest = it.getValue("latest").jsonPrimitive.contentOrNull,
                deprecated = it.getValue("deprecated").jsonPrimitive.boolean,
                message = it.getValue("message").jsonPrimitive.contentOrNull,
            )
        }

    private fun read(property: String) = Json.parseToJsonElement(File(System.getProperty(property)).readText())

    private fun JsonObject.text(key: String): String = getValue(key).jsonPrimitive.content

    /** The vector's camelCase options onto the class. */
    private fun options(o: JsonObject): SnippetOptions =
        SnippetOptions(o.text("planId"), o.text("tokenEndpoint")).apply {
            for ((key, value) in o) {
                val text = value.jsonPrimitive.content
                when (key) {
                    "surface" -> surface = text
                    "theme" -> theme = text
                    "locale" -> locale = text
                    "lingaraOrigin" -> lingaraOrigin = text
                    "title" -> title = text
                    "id" -> id = text
                    "nonce" -> nonce = text
                    "elementSrc" -> elementSrc = text
                    "planId", "tokenEndpoint" -> Unit
                    else -> error("an option the kit does not know: $key")
                }
            }
        }
}
