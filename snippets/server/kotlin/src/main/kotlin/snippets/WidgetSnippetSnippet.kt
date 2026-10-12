package snippets

import com.getlingara.embed.server.kotlin.SnippetOptions
import com.getlingara.embed.server.kotlin.widgetSnippet

/** The documentation site's widget-snippet example, on a page with a CSP nonce. */
fun page(
    planId: String,
    cspNonce: String,
): String {
    // lingara:begin widgetSnippet
    // planId is the lesson plan your server made for this player. The element
    // POSTs to tokenEndpoint with same-origin cookies: the token never appears
    // in the markup. Pass this request's CSP nonce if your page sets one.
    val html =
        widgetSnippet(
            SnippetOptions(planId, tokenEndpoint = "/lingara/token").apply {
                locale = "fr"
                nonce = cspNonce
            },
        )
    // lingara:end
    return html
}
