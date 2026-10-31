package com.getlingara.embed.java.conformance;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.LongNode;
import com.fasterxml.jackson.databind.node.NullNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.getlingara.embed.Command;
import com.getlingara.embed.CommandOutcome;
import com.getlingara.embed.EmbedEvent;
import com.getlingara.embed.EmbedException;
import com.getlingara.embed.Lingara_embed_uniffiKt;
import com.getlingara.embed.LogLevel;
import com.getlingara.embed.UpdateAvailable;
import com.getlingara.embed.java.LingaraEmbedJava;
import com.getlingara.embed.java.LingaraEmbedJavaException;
import java.io.IOException;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Deque;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;

/**
 * A case's {@code host:} script on the Java surface (ADR 1.10.26ae D9). Each step goes through
 * {@code LingaraEmbedJava}'s public methods only; commands are read with {@code commandFromJson}
 * and events written with {@code eventToJson}, so this harness never spells K1's JSON shape. A step
 * the core refuses becomes an {@code error} event, seen before anything still queued.
 */
final class Host {
  /** How long {@code close} waits for the one {@code Closed}. */
  private static final long CLOSE_WITHIN_MS = 2_000;

  final List<String> problems = new ArrayList<>();
  final List<JsonNode> matched = new ArrayList<>();
  private final String server;
  private final Deque<JsonNode> local = new ArrayDeque<>();
  private LingaraEmbedJava embed;
  private Long lastId;

  Host(String server) {
    this.server = server;
  }

  Host run(JsonNode testCase) {
    String id = testCase.path("id").asText();
    int n = 0;
    for (JsonNode step : testCase.path("host")) {
      Map.Entry<String, JsonNode> only = step.properties().iterator().next();
      try {
        step(only.getKey(), only.getValue());
      } catch (RuntimeException | IOException | InterruptedException e) {
        problems.add(id + ": host[" + n + "]." + only.getKey() + ": " + e.getMessage());
        break;
      }
      n++;
    }
    if (embed != null) {
      embed.close();
    }
    return this;
  }

  private LingaraEmbedJava embed() {
    if (embed == null) {
      throw new IllegalStateException("no start step ran");
    }
    return embed;
  }

  private void step(String name, JsonNode body) throws IOException, InterruptedException {
    switch (name) {
      case "start" -> start(body);
      case "provide_token" -> provide(body);
      case "command" -> command(body);
      case "poll_until" -> pollUntil(body);
      case "expect_no_event" -> expectNoEvent(body);
      case "stop_polling" -> Thread.sleep(body.path("for_ms").asLong());
      case "close" -> close();
      default -> throw new IllegalStateException("unknown step " + name);
    }
  }

  private static String text(JsonNode node, String key) {
    JsonNode value = node.get(key);
    return value == null || value.isNull() ? null : value.asText();
  }

  private void start(JsonNode body) {
    JsonNode c = body.path("config");
    LingaraEmbedJava.Builder builder =
        LingaraEmbedJava.builder()
            .baseUrl(server)
            .apiVersion(text(c, "api_version"))
            .resumeCursor(text(c, "resume_cursor"))
            .userAgentSuffix(text(c, "user_agent_suffix"));
    if (c.has("stream_on_start")) {
      builder.streamOnStart(c.get("stream_on_start").asBoolean());
    }
    if (c.has("log_level")) {
      builder.logLevel(LogLevel.valueOf(c.get("log_level").asText().toUpperCase(Locale.ROOT)));
    }
    limits(builder, c);
    embed = builder.build();
    String token = text(body, "initial_token");
    if (token != null && !token.equals("none")) {
      embed.provideTokenFor(token, 900);
    }
  }

  private static void limits(LingaraEmbedJava.Builder builder, JsonNode c) {
    if (c.has("queue_capacity")) {
      builder.queueCapacity(c.get("queue_capacity").asLong());
    }
    if (c.has("stream_idle_timeout_ms")) {
      builder.streamIdleTimeoutMs(c.get("stream_idle_timeout_ms").asLong());
    }
    List<String> types = new ArrayList<>();
    c.path("event_types").forEach(t -> types.add(t.asText()));
    builder.eventTypes(types);
  }

  private void provide(JsonNode body) {
    String token = text(body, "token");
    String at = text(body, "expires_at");
    JsonNode secs = body.get("expires_in_secs");
    if (at == null && secs == null) {
      throw new IllegalStateException("no expires_at or expires_in_secs");
    }
    UpdateAvailable update = updateFrom(body.get("update_available"));
    refusing(
        () -> {
          if (at != null && update != null) {
            embed().provideToken(token, at, update);
          } else if (at != null) {
            embed().provideToken(token, at);
          } else {
            embed().provideTokenFor(token, secs.asLong());
          }
        });
  }

  /**
   * A case's {@code update_available} as the typed notice (EK13, ADR 30.10.26p). A shape the record
   * cannot hold is dropped, as the core drops it.
   */
  private static UpdateAvailable updateFrom(JsonNode n) {
    if (n == null || !n.path("deprecated").isBoolean()) {
      return null;
    }
    JsonNode latest = n.path("latest");
    JsonNode message = n.path("message");
    if (!textOrAbsent(latest) || !textOrAbsent(message)) {
      return null;
    }
    return new UpdateAvailable(
        textOrNull(latest), n.get("deprecated").asBoolean(), textOrNull(message));
  }

  private static boolean textOrAbsent(JsonNode v) {
    return v.isMissingNode() || v.isNull() || v.isTextual();
  }

  private static String textOrNull(JsonNode v) {
    return v.isTextual() ? v.asText() : null;
  }

  /** A Command in its JSON shape; any {@code "$last"} is the latest opened session or dialogue. */
  private void command(JsonNode body) {
    Command command;
    try {
      command = Lingara_embed_uniffiKt.commandFromJson(substitute(body).toString());
    } catch (EmbedException e) {
      throw new IllegalStateException("not a Command: " + e.getMessage(), e);
    }
    boolean opens =
        command instanceof Command.PracticeStart
            || command instanceof Command.DialogueOpen
            || command instanceof Command.DialogueLoad;
    refusing(
        () -> {
          CommandOutcome outcome = embed().execute(command);
          if (opens && outcome instanceof CommandOutcome.Id id) {
            lastId = id.getId();
          }
        });
  }

  private void refusing(Runnable call) {
    try {
      call.run();
    } catch (LingaraEmbedJavaException e) {
      local.addLast(json(new EmbedEvent.Error(e.getError())));
    }
  }

  private JsonNode substitute(JsonNode value) {
    if (value.isTextual() && value.asText().equals("$last")) {
      return lastId == null ? NullNode.getInstance() : LongNode.valueOf(lastId);
    }
    if (value.isObject()) {
      ObjectNode out = Harness.JSON.createObjectNode();
      value.properties().forEach(e -> out.set(e.getKey(), substitute(e.getValue())));
      return out;
    }
    if (value.isArray()) {
      ArrayNode out = Harness.JSON.createArrayNode();
      value.forEach(item -> out.add(substitute(item)));
      return out;
    }
    return value;
  }

  private static JsonNode json(EmbedEvent event) {
    try {
      return Harness.JSON.readTree(Lingara_embed_uniffiKt.eventToJson(event));
    } catch (IOException e) {
      throw new IllegalStateException("eventToJson wrote no JSON", e);
    }
  }

  /** The next event, a refusal first. */
  private Optional<JsonNode> next() {
    if (!local.isEmpty()) {
      return Optional.of(local.removeFirst());
    }
    return embed == null ? Optional.empty() : embed.poll().map(Host::json);
  }

  /** Drains until an event matches or the deadline passes. */
  private Optional<JsonNode> find(String event, JsonNode wanted, long deadline)
      throws InterruptedException {
    while (System.currentTimeMillis() < deadline) {
      Optional<JsonNode> seen = next();
      if (seen.isEmpty()) {
        Thread.sleep(5);
      } else if (seen.get().path("type").asText().equals(event)
          && Matching.contains(seen.get(), wanted)) {
        return seen;
      }
    }
    return Optional.empty();
  }

  private void pollUntil(JsonNode body) throws InterruptedException {
    String event = body.path("event").asText();
    long within = body.path("within_ms").asLong();
    JsonNode wanted = body.get("match");
    matched.add(
        find(event, wanted, System.currentTimeMillis() + within)
            .orElseThrow(
                () ->
                    new IllegalStateException(
                        "no "
                            + event
                            + " event matching "
                            + wanted
                            + " within "
                            + within
                            + " ms")));
  }

  private void expectNoEvent(JsonNode body) throws InterruptedException {
    String event = body.path("event").asText();
    long period = body.path("for_ms").asLong();
    Optional<JsonNode> seen = find(event, body.get("match"), System.currentTimeMillis() + period);
    if (seen.isPresent()) {
      throw new IllegalStateException("an unexpected " + event + " event: " + seen.get());
    }
  }

  private void close() throws InterruptedException {
    embed().shutdown();
    find("closed", null, System.currentTimeMillis() + CLOSE_WITHIN_MS)
        .orElseThrow(() -> new IllegalStateException("no closed event after close"));
    Optional<EmbedEvent> after = embed().poll();
    if (after.isPresent()) {
      throw new IllegalStateException("an event after closed: " + after.get());
    }
  }
}
