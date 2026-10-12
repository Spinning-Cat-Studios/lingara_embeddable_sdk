package com.getlingara.embed.server.conformance;

import com.fasterxml.jackson.databind.JsonNode;
import com.getlingara.client.events.UnknownEvent;
import com.getlingara.embed.server.PlayerRouter;
import com.getlingara.embed.server.RoutedEvent;
import com.sun.net.httpserver.HttpExchange;
import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/**
 * The webhook receiver the conformance server posts to. Delivery {@code n} of the armed case is
 * answered through a {@link PlayerRouter} built from that item's {@code router.secrets}, whose
 * {@code onEvent} records what it saw and throws when the item's {@code handler} is {@code raise}.
 */
final class Receiver {
  /** What one delivery's {@code onEvent} saw. */
  record Seen(String playerRef, String eventId, boolean unknown) {}

  /** The armed case, as the receiver needs it. */
  static final class Armed {
    final List<JsonNode> deliver;
    final Recording directory;
    final List<Seen> seen = new ArrayList<>();
    int received;
    int answered;

    Armed(List<JsonNode> deliver, Recording directory) {
      this.deliver = deliver;
      this.directory = directory;
      deliver.forEach(d -> seen.add(null));
    }
  }

  /** {@code primary} and {@code secondary}, from {@code LINGARA_EMBED_WEBHOOK_SECRETS}. */
  private final Map<String, String> secrets;

  final Recording.Sink sink;
  private Armed armed;

  Receiver(Map<String, String> secrets, Recording.Sink sink) {
    this.secrets = secrets;
    this.sink = sink;
  }

  synchronized void arm(Armed next) {
    armed = next;
  }

  synchronized boolean answered(int count) {
    return armed == null || armed.answered >= count;
  }

  synchronized List<Seen> seen() {
    return armed == null ? List.of() : new ArrayList<>(armed.seen);
  }

  /** The router delivery {@code item} is answered through. */
  PlayerRouter router(JsonNode item, Recording directory) {
    List<String> names = new ArrayList<>();
    JsonNode named = item.path("router").path("secrets");
    if (named.isArray()) {
      named.forEach(n -> names.add(n.asText()));
    } else {
      names.add("primary");
    }
    String[] values = new String[names.size()];
    for (int i = 0; i < values.length; i++) {
      values[i] = secrets.get(names.get(i));
      if (values[i] == null) {
        throw new IllegalArgumentException("no secret named " + names.get(i));
      }
    }
    return PlayerRouter.of(directory, values).withOnError(e -> sink.push("on_error: " + e));
  }

  void deliver(HttpExchange exchange) throws IOException {
    byte[] body = exchange.getRequestBody().readAllBytes();
    int n;
    JsonNode item;
    Recording directory;
    synchronized (this) {
      if (armed == null) {
        exchange.sendResponseHeaders(409, -1);
        exchange.close();
        return;
      }
      n = armed.received++;
      item =
          n < armed.deliver.size()
              ? armed.deliver.get(n)
              : com.fasterxml.jackson.databind.node.NullNode.instance;
      directory = armed.directory;
    }
    int status = answer(n, item, directory, body, exchange);
    synchronized (this) {
      if (armed != null) {
        armed.answered++;
      }
    }
    exchange.sendResponseHeaders(status, -1);
    exchange.close();
  }

  private int answer(
      int n, JsonNode item, Recording directory, byte[] body, HttpExchange exchange) {
    PlayerRouter router;
    try {
      router = router(item, directory);
    } catch (RuntimeException e) {
      sink.push("harness: " + e.getMessage());
      return 500;
    }
    boolean raise = "raise".equals(item.path("handler").asText());
    return router.handle(body, exchange.getRequestHeaders(), routed -> record(n, routed, raise));
  }

  private void record(int n, RoutedEvent routed, boolean raise) {
    Seen seen =
        new Seen(
            routed.playerRef().orElse(null),
            routed.event().id(),
            routed.event() instanceof UnknownEvent);
    synchronized (this) {
      if (armed != null && n < armed.seen.size()) {
        armed.seen.set(n, seen);
      }
    }
    if (raise) {
      throw new IllegalStateException("the conformance handler raised, as the case asked");
    }
  }
}
