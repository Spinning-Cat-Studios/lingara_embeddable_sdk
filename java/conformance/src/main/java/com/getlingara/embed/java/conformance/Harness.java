package com.getlingara.embed.java.conformance;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.File;
import java.io.IOException;
import java.io.PrintWriter;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

/**
 * The EK harness on the Java surface (ADR 1.10.26ae D9). {@code conformance-server run} starts it
 * with {@code LINGARA_EMBED_SERVER_URL} and {@code LINGARA_EMBED_RESULTS}. For each case under
 * {@code conformance/ek/cases/} it fetches the case, arms it, runs its host script through {@code
 * LingaraEmbedJava}, finishes it, and writes one result line merging the server's verdict with its
 * own. It reports every case: there is no skip.
 */
public final class Harness {
  /** The language this harness reports: make/java.mk's EK_LANGS entry. */
  private static final String LANG = "java";

  static final ObjectMapper JSON = new ObjectMapper();
  private static final HttpClient HTTP = HttpClient.newHttpClient();

  private Harness() {}

  public static void main(String[] args) throws IOException {
    String server = required("LINGARA_EMBED_SERVER_URL");
    String results = required("LINGARA_EMBED_RESULTS");
    String casesDir = System.getenv().getOrDefault("LINGARA_EMBED_CASES", "conformance/ek/cases");
    // A failing case is in the results, which the server judges; the exit code says only that
    // every case was run and reported.
    try (PrintWriter out = new PrintWriter(new File(results), StandardCharsets.UTF_8)) {
      for (String id : caseIds(new File(casesDir))) {
        out.println(runCase(server, id));
      }
    }
  }

  private static String required(String name) {
    String value = System.getenv(name);
    if (value == null) {
      throw new IllegalStateException(name + " is not set");
    }
    return value;
  }

  /** {@code <group>/<slug>.yaml} → {@code <group>.<slug>}, sorted. */
  static List<String> caseIds(File dir) {
    List<String> ids = new ArrayList<>();
    File[] groups = dir.listFiles(File::isDirectory);
    for (File group : groups == null ? new File[0] : groups) {
      File[] files = group.listFiles((d, name) -> name.endsWith(".yaml"));
      for (File file : files == null ? new File[0] : files) {
        String slug = file.getName().substring(0, file.getName().length() - ".yaml".length());
        ids.add(group.getName() + "." + slug);
      }
    }
    ids.sort(null);
    return ids;
  }

  static String request(String url, String body) throws IOException, InterruptedException {
    HttpRequest.Builder builder = HttpRequest.newBuilder(URI.create(url));
    HttpRequest req =
        body == null
            ? builder.GET().build()
            : builder
                .header("content-type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(body))
                .build();
    HttpResponse<String> response = HTTP.send(req, HttpResponse.BodyHandlers.ofString());
    if (response.statusCode() / 100 != 2) {
      throw new IOException(url + " answered " + response.statusCode());
    }
    return response.body();
  }

  /** One case end to end. A failure to reach the server is a mismatch, so it is still reported. */
  private static String runCase(String server, String id) {
    String control = server + "/__conformance/%s/" + id;
    List<String> mismatches = new ArrayList<>();
    JsonNode testCase;
    try {
      testCase = JSON.readTree(request(String.format(control, "cases"), null));
    } catch (IOException | InterruptedException e) {
      mismatches.add("cannot read the case: " + e.getMessage());
      return result(id, mismatches);
    }
    try {
      request(String.format(control, "arm"), "{}");
    } catch (IOException | InterruptedException e) {
      mismatches.add("cannot arm: " + e.getMessage());
    }
    Host host = new Host(server).run(testCase);
    mismatches.addAll(host.problems);
    JsonNode want = testCase.path("expect").path("events");
    if (want.isArray()) {
      JsonNode got = Matching.withoutNulls(JSON.valueToTree(host.matched));
      if (!got.equals(Matching.withoutNulls(want))) {
        mismatches.add(id + ": the host saw " + got + ", expected " + Matching.withoutNulls(want));
      }
    }
    finish(String.format(control, "finish"), mismatches);
    return result(id, mismatches);
  }

  private static void finish(String url, List<String> mismatches) {
    try {
      JSON.readTree(request(url, "{}"))
          .path("mismatches")
          .forEach(m -> mismatches.add(m.isTextual() ? m.asText() : m.toString()));
    } catch (IOException | InterruptedException e) {
      mismatches.add("cannot finish: " + e.getMessage());
    }
  }

  private static String result(String id, List<String> mismatches) {
    ObjectNode line = JSON.createObjectNode();
    line.put("case", id);
    line.put("lang", LANG);
    line.put("result", mismatches.isEmpty() ? "pass" : "fail");
    ArrayNode list = line.putArray("mismatches");
    mismatches.forEach(list::add);
    return line.toString();
  }
}
