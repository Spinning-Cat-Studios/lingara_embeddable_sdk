package com.getlingara.embed.server;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.File;
import java.util.Map;
import org.junit.jupiter.api.Test;

class VectorsTest {
  private static final ObjectMapper JSON = new ObjectMapper();

  /**
   * 1.10.26af AC9: every {@code snippet.json} vector through {@link WidgetSnippet#render}, and
   * every {@code token-handoff.json} vector through the internal {@code handoffBody}, gives its
   * expected answer, with {@code {{version}}} read as this kit's VERSION.
   */
  @Test
  void everySnippetAndHandoffVector() throws Exception {
    String version = System.getProperty("lingara.version");
    JsonNode snippets = JSON.readTree(new File(System.getProperty("lingara.snippetVectors")));
    int checked = 0;
    for (JsonNode v : snippets.get("cases")) {
      String name = v.get("name").asText();
      SnippetOptions options = options(v.get("options"));
      JsonNode expect = v.get("expect");
      if (expect.has("html")) {
        String html = expect.get("html").asText().replace("{{version}}", version);
        assertEquals(html, WidgetSnippet.render(options), name);
      } else {
        SnippetException e =
            assertThrows(SnippetException.class, () -> WidgetSnippet.render(options), name);
        assertEquals(expect.get("refused").asText(), e.field(), name);
      }
      checked++;
    }
    assertTrue(checked > 0, "snippet.json holds cases");

    JsonNode handoffs = JSON.readTree(new File(System.getProperty("lingara.handoffVectors")));
    assertTrue(handoffs.size() > 0, "token-handoff.json holds cases");
    for (JsonNode v : handoffs) {
      JsonNode in = v.get("input");
      String body =
          TokenHandoff.handoffBody(in.get("token").asText(), in.get("expires_at").asText());
      assertEquals(v.get("body").asText(), body, v.get("name").asText());
    }
  }

  /** The vector's camelCase options through the builder. */
  private static SnippetOptions options(JsonNode o) {
    SnippetOptions.Builder b = SnippetOptions.builder(text(o, "planId"), text(o, "tokenEndpoint"));
    for (Map.Entry<String, JsonNode> f : o.properties()) {
      set(b, f.getKey(), f.getValue().asText());
    }
    return b.build();
  }

  private static void set(SnippetOptions.Builder b, String key, String value) {
    switch (key) {
      case "surface" -> b.surface(value);
      case "theme" -> b.theme(value);
      case "locale" -> b.locale(value);
      case "lingaraOrigin" -> b.lingaraOrigin(value);
      case "title" -> b.title(value);
      case "id" -> b.id(value);
      case "nonce" -> b.nonce(value);
      case "elementSrc" -> b.elementSrc(value);
      case "planId", "tokenEndpoint" -> {
        // Given to the builder.
      }
      default -> throw new IllegalArgumentException("an option the kit does not know: " + key);
    }
  }

  private static String text(JsonNode o, String key) {
    return o.has(key) ? o.get(key).asText() : null;
  }
}
