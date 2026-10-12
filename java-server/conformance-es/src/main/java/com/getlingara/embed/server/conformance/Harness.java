package com.getlingara.embed.server.conformance;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.getlingara.client.LingaraClient;
import com.getlingara.embed.server.EmbedServer;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.PrintWriter;
import java.net.InetSocketAddress;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.Executors;
import java.util.stream.Stream;

/**
 * The ES harness on the Java server kit (ADR 1.10.26af D13): every ES case through {@code
 * com.getlingara.embed.server} and nothing else.
 *
 * <p>{@code conformance-server run --tier es --lang java-server -- <this>} starts it with the
 * server's URL, the results path, the receiver's two webhook secrets and the visibly fake client
 * credentials. It binds its webhook receiver to port 0, prints {@code listening <port>}, and then
 * for each case: fetches it, builds a fresh client, server and directory, arms it, runs the {@code
 * kit:} jobs, waits for every delivery, judges what only the caller sees, finishes, and writes one
 * result line. There is no skip.
 */
public final class Harness {
  /** The language this harness reports: make/java-server.mk's ES_LANGS entry. */
  private static final String LANG = "java-server";

  private static final List<String> SECRET_PREFIXES = List.of("lgr_et_", "lgr_cs_", "lgr_whsec_");
  private static final long WAIT_MILLIS = 15_000;

  private final HttpClient http = HttpClient.newHttpClient();
  private final Receiver receiver;
  private final String server = env("LINGARA_EMBED_SERVER_URL");

  private Harness(Receiver receiver) {
    this.receiver = receiver;
  }

  /**
   * Runs every case, then exits: the results file carries the verdicts.
   *
   * @param args unused
   * @throws Exception when the receiver cannot bind or the results cannot be written
   */
  public static void main(String[] args) throws Exception {
    Recording.Sink sink = new Recording.Sink();
    sink.captureLogs();
    String[] named = env("LINGARA_EMBED_WEBHOOK_SECRETS").split(",");
    Map<String, String> secrets = new HashMap<>();
    secrets.put("primary", named[0]);
    secrets.put("secondary", named.length > 1 ? named[1] : null);
    Receiver receiver = new Receiver(secrets, sink);
    HttpServer listener = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
    listener.createContext("/", receiver::deliver);
    listener.setExecutor(Executors.newCachedThreadPool());
    listener.start();
    System.out.println("listening " + listener.getAddress().getPort());
    System.out.flush();

    Harness harness = new Harness(receiver);
    String cases = System.getenv().getOrDefault("LINGARA_EMBED_CASES", "conformance/es/cases");
    try (PrintWriter out =
        new PrintWriter(
            Files.newBufferedWriter(
                Path.of(env("LINGARA_EMBED_RESULTS")), StandardCharsets.UTF_8))) {
      for (String id : caseIds(Path.of(cases))) {
        out.println(harness.runCase(id));
        out.flush();
      }
    }
    listener.stop(0);
    System.exit(0);
  }

  private static String env(String name) {
    String value = System.getenv(name);
    if (value == null) {
      throw new IllegalStateException(name + " is not set");
    }
    return value;
  }

  /** {@code <group>/<slug>.yaml} to {@code <group>.<slug>}, sorted. */
  private static List<String> caseIds(Path dir) throws IOException {
    List<String> ids = new ArrayList<>();
    try (Stream<Path> files = Files.walk(dir, 2)) {
      files
          .filter(p -> p.toString().endsWith(".yaml") && !p.getParent().equals(dir))
          .forEach(p -> ids.add(p.getParent().getFileName() + "." + stem(p)));
    }
    ids.sort(null);
    return ids;
  }

  private static String stem(Path p) {
    String name = p.getFileName().toString();
    return name.substring(0, name.length() - ".yaml".length());
  }

  private String runCase(String id) throws IOException, InterruptedException {
    JsonNode c;
    try {
      c = Kit.JSON.readTree(call("GET", control("cases", id), null));
    } catch (IOException e) {
      return result(id, List.of("cannot read the case: " + e.getMessage()));
    }
    Map<String, String> seed = new HashMap<>();
    c.path("directory")
        .path("seed")
        .properties()
        .forEach(e -> seed.put(e.getKey(), e.getValue().asText()));
    Recording directory = new Recording(seed);
    List<JsonNode> deliver = new ArrayList<>();
    c.path("deliver").forEach(deliver::add);
    receiver.arm(new Receiver.Armed(deliver, directory));
    receiver.sink.take();
    EmbedServer embed = EmbedServer.of(client(), directory);

    List<String> mismatches = new ArrayList<>();
    try {
      call("POST", control("arm", id), "");
    } catch (IOException e) {
      mismatches.add("cannot arm: " + e.getMessage());
    }
    Kit kit = new Kit();
    kit.run(embed, c.path("kit"));
    List<Receiver.Seen> seen = waitForDeliveries(deliver.size());
    mismatches.addAll(kit.judge(id, c.path("expect"), directory));
    mismatches.addAll(handlerMismatches(id, deliver, seen));
    if (kit.render) {
      mismatches.addAll(renderMismatches(id, embed, directory, kit));
    }
    mismatches.addAll(finish(id, seen));
    receiver.arm(null);
    return result(id, mismatches);
  }

  private LingaraClient client() {
    return LingaraClient.builder()
        .clientCredentials(env("LINGARA_EMBED_CLIENT_ID"), env("LINGARA_EMBED_CLIENT_SECRET"))
        .baseUrl(URI.create(server))
        .tokenUrl(URI.create(server + "/oauth/token"))
        .build();
  }

  private List<String> finish(String id, List<Receiver.Seen> seen) throws InterruptedException {
    ObjectNode body = Kit.JSON.createObjectNode();
    ArrayNode handlers = body.putArray("handlers");
    seen.forEach(s -> handlers.add(s == null ? null : s.playerRef()));
    List<String> out = new ArrayList<>();
    try {
      JsonNode answer = Kit.JSON.readTree(call("POST", control("finish", id), body.toString()));
      answer.path("mismatches").forEach(m -> out.add(m.asText()));
    } catch (IOException e) {
      out.add("cannot finish: " + e.getMessage());
    }
    return out;
  }

  private String control(String verb, String id) {
    return server + "/__conformance/" + verb + "/" + id;
  }

  private String call(String method, String url, String json)
      throws IOException, InterruptedException {
    HttpRequest.BodyPublisher publisher =
        json == null
            ? HttpRequest.BodyPublishers.noBody()
            : HttpRequest.BodyPublishers.ofString(json);
    HttpRequest request =
        HttpRequest.newBuilder(URI.create(url))
            .method(method, publisher)
            .header("content-type", "application/json")
            .build();
    HttpResponse<String> response = http.send(request, HttpResponse.BodyHandlers.ofString());
    if (response.statusCode() >= 400) {
      throw new IOException(method + " " + url + " answered " + response.statusCode());
    }
    return response.body();
  }

  /** Until every delivery has been answered, or 15 s; then what each saw. */
  private List<Receiver.Seen> waitForDeliveries(int count) throws InterruptedException {
    long deadline = System.currentTimeMillis() + WAIT_MILLIS;
    while (!receiver.answered(count) && System.currentTimeMillis() < deadline) {
      Thread.sleep(20);
    }
    return receiver.seen();
  }

  /** Each delivery's {@code expect.handler} against what its {@code onEvent} saw. */
  private static List<String> handlerMismatches(
      String id, List<JsonNode> deliver, List<Receiver.Seen> seen) {
    List<String> found = new ArrayList<>();
    for (int n = 0; n < deliver.size(); n++) {
      JsonNode want = deliver.get(n).path("expect").path("handler");
      Receiver.Seen got = n < seen.size() ? seen.get(n) : null;
      if (!handlerMatches(want, got)) {
        found.add(id + ": fail handler: deliver[" + n + "] saw " + got + ", expected " + want);
      }
    }
    return found;
  }

  private static boolean handlerMatches(JsonNode want, Receiver.Seen got) {
    if (want.isTextual()) {
      return "not_called".equals(want.asText()) && got == null;
    }
    if (got == null || !want.isObject()) {
      return false;
    }
    boolean player = !want.has("player_ref") || sameText(want.get("player_ref"), got.playerRef());
    boolean event = !want.has("event_id") || sameText(want.get("event_id"), got.eventId());
    boolean unknown = !want.has("unknown") || want.get("unknown").asBoolean() == got.unknown();
    return player && event && unknown;
  }

  /** A case value against an observed one: JSON null is "none". */
  private static boolean sameText(JsonNode want, String got) {
    return want.isNull() ? got == null : want.asText().equals(got);
  }

  /** ES5: no rendering of a kit object, and no captured log line, carries a secret prefix. */
  private List<String> renderMismatches(
      String id, EmbedServer embed, Recording directory, Kit kit) {
    ObjectNode both = Kit.JSON.createObjectNode();
    both.putObject("router").putArray("secrets").add("primary").add("secondary");
    List<String> renderings = new ArrayList<>();
    renderings.add(embed.toString());
    renderings.add(receiver.router(both, directory).toString());
    renderings.add(directory.toString());
    renderings.add(String.valueOf(kit.minted));
    if (kit.minted != null) {
      renderings.add(kit.minted.token().toString());
    }
    renderings.addAll(receiver.sink.take());
    long leaks =
        renderings.stream().filter(r -> SECRET_PREFIXES.stream().anyMatch(r::contains)).count();
    return leaks == 0
        ? List.of()
        : List.of(id + ": fail render: " + leaks + " rendering(s) carry a secret prefix");
  }

  private static String result(String id, List<String> mismatches) {
    ObjectNode line = Kit.JSON.createObjectNode();
    line.put("case", id).put("lang", LANG).put("result", mismatches.isEmpty() ? "pass" : "fail");
    mismatches.forEach(line.putArray("mismatches")::add);
    return line.toString();
  }
}
