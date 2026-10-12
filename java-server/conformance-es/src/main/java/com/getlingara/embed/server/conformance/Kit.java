package com.getlingara.embed.server.conformance;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.getlingara.client.ApiException;
import com.getlingara.client.MaintenanceException;
import com.getlingara.client.MintedToken;
import com.getlingara.client.OAuthException;
import com.getlingara.client.TransportException;
import com.getlingara.embed.server.EmbedServer;
import com.getlingara.embed.server.MintOptions;
import com.getlingara.embed.server.TokenHandoff;
import java.io.UncheckedIOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/**
 * A case's {@code kit:} jobs through {@link EmbedServer}, and the comparisons only the kit's caller
 * can make (ES {@code CONTRACT.md}, What a harness reports).
 */
final class Kit {
  static final ObjectMapper JSON = new ObjectMapper();
  private static final List<String> KEYS =
      List.of("minted", "handoff", "error", "directory", "directory_lookups");

  /** What the jobs left behind. */
  MintedToken minted;

  String handoff;
  JsonNode error;
  boolean render;

  /** Every job in order; a raised error is recorded and the next job runs. */
  void run(EmbedServer server, JsonNode jobs) {
    for (JsonNode job : jobs) {
      try {
        step(server, job);
      } catch (RuntimeException e) {
        error = raised(e);
      }
    }
  }

  private void step(EmbedServer server, JsonNode job) {
    if (job.has("mint")) {
      JsonNode m = job.get("mint");
      minted = server.mintToken(m.path("player_ref").asText(), options(m));
    } else if (job.has("delete")) {
      server.deletePlayer(job.get("delete").path("player_ref").asText());
    } else if (job.has("handoff")) {
      handoff = minted == null ? null : TokenHandoff.body(minted);
    } else {
      render = true;
    }
  }

  private static MintOptions options(JsonNode m) {
    MintOptions o = MintOptions.none();
    if (m.has("scopes")) {
      List<String> scopes = new ArrayList<>();
      m.get("scopes").forEach(s -> scopes.add(s.asText()));
      o = o.withScopes(scopes);
    }
    return m.hasNonNull("origin") ? o.withOrigin(m.get("origin").asText()) : o;
  }

  /** {@code {status, code}}: an API error's, an OAuth error's {@code error}, maintenance. */
  private static JsonNode raised(RuntimeException e) {
    ObjectNode out = JSON.createObjectNode();
    if (e instanceof ApiException a) {
      out.put("status", a.status()).put("code", a.code());
    } else if (e instanceof OAuthException o) {
      out.put("status", o.status()).put("code", o.error());
    } else if (e instanceof MaintenanceException) {
      out.put("status", 503).put("code", "maintenance");
    } else if (e instanceof TransportException t) {
      out.put("status", 0).put("code", t.kind().wireName());
    } else {
      out.put("status", 0).put("code", e.getClass().getSimpleName());
    }
    return out;
  }

  /** {@code value} with every null object member dropped, at any depth, and numbers normalised. */
  static JsonNode withoutNulls(JsonNode value) {
    if (value == null) {
      return null;
    }
    JsonNode copy = reparse(value);
    strip(copy);
    return copy;
  }

  private static void strip(JsonNode node) {
    if (node instanceof ObjectNode o) {
      List<String> drop = new ArrayList<>();
      for (Map.Entry<String, JsonNode> p : o.properties()) {
        if (p.getValue().isNull()) {
          drop.add(p.getKey());
        }
      }
      o.remove(drop);
    }
    node.forEach(Kit::strip);
  }

  private static JsonNode reparse(Object value) {
    try {
      return JSON.readTree(JSON.writeValueAsString(value));
    } catch (com.fasterxml.jackson.core.JsonProcessingException e) {
      throw new UncheckedIOException(e);
    }
  }

  /** The six fields in wire form, the token read through its accessor. */
  private JsonNode wire() {
    if (minted == null) {
      return null;
    }
    ObjectNode out = JSON.createObjectNode();
    out.put("token", minted.token().exposeSecret()).put("expires_at", minted.expiresAt());
    out.put("expires_in", minted.expiresIn().getSeconds()).put("subject", minted.subject());
    minted.scopes().forEach(out.putArray("scopes")::add);
    return out.put("account_linked", minted.accountLinked());
  }

  private JsonNode got(String key, Recording directory) {
    return switch (key) {
      case "minted" -> wire();
      case "handoff" -> handoff == null ? null : JSON.getNodeFactory().textNode(handoff);
      case "error" -> error;
      case "directory" -> reparse(directory.snapshot());
      default -> JSON.getNodeFactory().numberNode(directory.lookups());
    };
  }

  /** Each compared key, a {@code fail <key>} line when it differs. */
  List<String> judge(String id, JsonNode expect, Recording directory) {
    List<String> found = new ArrayList<>();
    for (String key : KEYS) {
      JsonNode want = withoutNulls(expect.get(key));
      JsonNode have = withoutNulls(got(key, directory));
      // A key the case names must match; an error it did not expect is a mismatch too.
      boolean differs = want != null ? !want.equals(have) : "error".equals(key) && have != null;
      if (differs) {
        found.add(id + ": fail " + key + ": got " + show(have) + ", expected " + show(want));
      }
    }
    return found;
  }

  private static String show(JsonNode v) {
    return v == null ? "nothing" : v.toString();
  }
}
