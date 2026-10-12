package com.getlingara.embed.server;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.getlingara.client.events.Webhook;
import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.time.Clock;
import java.time.Instant;
import java.time.ZoneOffset;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.TreeMap;
import org.junit.jupiter.api.Test;

class PlayerRouterTest {
  private static final ObjectMapper JSON = new ObjectMapper();

  /** A directory that counts lookups, and fails them when asked to. */
  private static final class Counting implements PlayerDirectory {
    final MemoryPlayerDirectory inner = new MemoryPlayerDirectory();
    final boolean fail;
    int lookups;

    Counting(boolean fail) {
      this.fail = fail;
      inner.remember("p1", "lgr_sub_conformance");
    }

    @Override
    public void remember(String playerRef, String subject) {
      inner.remember(playerRef, subject);
    }

    @Override
    public Optional<String> playerRefFor(String subject) {
      lookups++;
      if (fail) {
        throw new IllegalStateException("the store is down");
      }
      return inner.playerRefFor(subject);
    }

    @Override
    public void forget(String playerRef) {
      inner.forget(playerRef);
    }
  }

  /** One E7 signature vector: its verifier at the vector's own now, its body and headers. */
  private record Vector(Webhook webhook, byte[] body, Map<String, List<String>> headers) {}

  private static Vector vector(String name) throws IOException {
    JsonNode all = JSON.readTree(new File(System.getProperty("lingara.webhookVectors")));
    for (JsonNode v : all.get("vectors")) {
      if (v.get("name").asText().equals(name)) {
        List<String> secrets = new ArrayList<>();
        v.get("secrets").forEach(s -> secrets.add(s.asText()));
        Clock at = Clock.fixed(Instant.ofEpochSecond(v.get("now").asLong()), ZoneOffset.UTC);
        Map<String, List<String>> headers = new TreeMap<>();
        v.get("headers")
            .properties()
            .forEach(h -> headers.put(h.getKey(), List.of(h.getValue().asText())));
        byte[] body = v.get("body").asText().getBytes(StandardCharsets.UTF_8);
        return new Vector(Webhook.of(secrets, at), body, headers);
      }
    }
    throw new IllegalArgumentException("no vector " + name);
  }

  /**
   * 1.10.26af AC17: through {@code handle}, a tampered body is 401 and never reaches the directory
   * or {@code onEvent}; a valid event reaches {@code onEvent} with the remembered player_ref; an
   * unknown subject arrives with none; a raising handler and a raising directory are each 500 and
   * reach {@code onError}; a valid signature over a body that is not an event is 400.
   */
  @Test
  void handleVerifiesBeforeResolving() throws Exception {
    List<Optional<String>> seen = new ArrayList<>();
    EventHandler record = routed -> seen.add(routed.playerRef());

    Vector tampered = vector("tampered-body");
    Counting dir = new Counting(false);
    PlayerRouter router = PlayerRouter.of(dir, tampered.webhook());
    assertEquals(401, router.handle(tampered.body(), tampered.headers(), record));
    assertEquals(0, dir.lookups);
    assertTrue(seen.isEmpty());

    Vector valid = vector("valid-single-signature");
    router = PlayerRouter.of(dir, valid.webhook());
    assertEquals(204, router.handle(valid.body(), valid.headers(), record));
    assertEquals(1, dir.lookups);
    assertEquals(List.of(Optional.of("p1")), seen);

    dir.forget("p1");
    seen.clear();
    assertEquals(204, router.handle(valid.body(), valid.headers(), record));
    assertEquals(List.of(Optional.empty()), seen, "an unknown subject routes to no player");

    List<String> errors = new ArrayList<>();
    router =
        PlayerRouter.of(new Counting(false), valid.webhook())
            .withOnError(e -> errors.add(e.getMessage()));
    EventHandler raise =
        routed -> {
          throw new IllegalStateException("the handler broke");
        };
    assertEquals(500, router.handle(valid.body(), valid.headers(), raise));
    assertEquals(List.of("the handler broke"), errors);

    errors.clear();
    seen.clear();
    router =
        PlayerRouter.of(new Counting(true), valid.webhook())
            .withOnError(e -> errors.add(e.getMessage()));
    assertEquals(500, router.handle(valid.body(), valid.headers(), record));
    assertEquals(List.of("the store is down"), errors);
    assertTrue(seen.isEmpty(), "a failed lookup never calls onEvent");

    Vector notJson = vector("body-not-json");
    router = PlayerRouter.of(new Counting(false), notJson.webhook());
    assertEquals(400, router.handle(notJson.body(), notJson.headers(), record));
  }
}
