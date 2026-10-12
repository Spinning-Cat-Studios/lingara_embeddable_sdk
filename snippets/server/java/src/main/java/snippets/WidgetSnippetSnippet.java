package snippets;

import com.getlingara.embed.server.SnippetOptions;
import com.getlingara.embed.server.WidgetSnippet;

/** The documentation site's widget-snippet example. */
public final class WidgetSnippetSnippet {
  private WidgetSnippetSnippet() {}

  /**
   * The widget for one plan, on a page with a CSP nonce.
   *
   * @param planId the plan your server made for this player
   * @param cspNonce this request's nonce
   * @return the HTML
   */
  public static String page(String planId, String cspNonce) {
    // lingara:begin widgetSnippet
    // planId is the lesson plan your server made for this player. The element
    // POSTs to tokenEndpoint with same-origin cookies: the token never appears
    // in the markup. Pass this request's CSP nonce if your page sets one.
    String html =
        WidgetSnippet.render(
            SnippetOptions.builder(planId, "/lingara/token").locale("fr").nonce(cspNonce).build());
    // lingara:end
    return html;
  }
}
