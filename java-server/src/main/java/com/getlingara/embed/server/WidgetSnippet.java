package com.getlingara.embed.server;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;

/**
 * The widget snippet (ADR 1.10.26af D7, ES4): the {@code <lingara-embed>} element and its two
 * module scripts, rendered identically by every server kit. The markup is the widget's own; the
 * token never appears in it, only the endpoint the element asks at run time, with {@code
 * credentials: "same-origin"}.
 */
public final class WidgetSnippet {
  /** This kit's release, which the default element URL names. */
  static final String VERSION = readVersion();

  /** The script-tag build of this release's element. */
  public static final String DEFAULT_ELEMENT_SRC =
      "https://cdn.jsdelivr.net/npm/@lingara/embed@" + VERSION + "/dist/lingara-embed.min.js";

  private WidgetSnippet() {}

  /**
   * Returns the three lines to paste into your page, with no trailing newline.
   *
   * @param o the options
   * @return the HTML
   * @throws SnippetException naming the first option that breaks its rule
   */
  public static String render(SnippetOptions o) {
    String surface = o.surface == null ? "practice" : o.surface;
    String id = o.id == null ? "lingara-embed" : o.id;
    String src = o.elementSrc == null ? DEFAULT_ELEMENT_SRC : o.elementSrc;
    String refused = SnippetRules.firstRefused(o, surface, id);
    if (refused != null) {
      throw new SnippetException(refused);
    }
    StringBuilder html = new StringBuilder("<lingara-embed");
    attribute(html, "id", id);
    attribute(html, "surface", surface);
    attribute(html, "plan-id", o.planId);
    attribute(html, "theme", o.theme);
    attribute(html, "locale", o.locale);
    attribute(html, "lingara-origin", o.lingaraOrigin);
    attribute(html, "title", o.title);
    String nonce = o.nonce == null ? "" : " nonce=\"" + attr(o.nonce) + "\"";
    return html.append("></lingara-embed>\n<script type=\"module\"")
        .append(nonce)
        .append(" src=\"")
        .append(attr(src))
        .append("\"></script>\n<script type=\"module\"")
        .append(nonce)
        .append(">document.getElementById(")
        .append(scriptString(id))
        .append(").tokenProvider = async () => (await fetch(")
        .append(scriptString(o.tokenEndpoint))
        .append(", { method: \"POST\", credentials: \"same-origin\" })).json();</script>")
        .toString();
  }

  private static void attribute(StringBuilder html, String name, String value) {
    if (value != null) {
      html.append(' ').append(name).append("=\"").append(attr(value)).append('"');
    }
  }

  /** An attribute value: the five characters, and nothing else. */
  static String attr(String value) {
    StringBuilder out = new StringBuilder(value.length());
    value
        .codePoints()
        .forEach(
            c -> {
              switch (c) {
                case '&' -> out.append("&amp;");
                case '<' -> out.append("&lt;");
                case '>' -> out.append("&gt;");
                case '"' -> out.append("&quot;");
                case '\'' -> out.append("&#39;");
                default -> out.appendCodePoint(c);
              }
            });
    return out.toString();
  }

  /** A JSON string literal that can never close the script or open a comment. */
  static String scriptString(String value) {
    StringBuilder out = new StringBuilder("\"");
    value
        .codePoints()
        .forEach(
            c -> {
              switch (c) {
                case '"' -> out.append("\\\"");
                case '\\' -> out.append("\\\\");
                case '<' -> out.append("\\u003c");
                case '>' -> out.append("\\u003e");
                case '&' -> out.append("\\u0026");
                case 0x2028 -> out.append("\\u2028");
                case 0x2029 -> out.append("\\u2029");
                default -> out.appendCodePoint(c);
              }
            });
    return out.append('"').toString();
  }

  private static String readVersion() {
    try (InputStream in = WidgetSnippet.class.getResourceAsStream("version.txt")) {
      if (in == null) {
        throw new IllegalStateException("the kit's version.txt resource is missing");
      }
      return new String(in.readAllBytes(), StandardCharsets.UTF_8).trim();
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }
}
