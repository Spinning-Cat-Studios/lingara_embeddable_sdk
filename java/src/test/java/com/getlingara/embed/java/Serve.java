package com.getlingara.embed.java;

import java.io.BufferedReader;
import java.io.File;
import java.io.IOException;
import java.io.InputStreamReader;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;

/** {@code conformance-server serve} over the EK cases, built by {@code make test-java}. */
final class Serve implements AutoCloseable {
  private final Process process;
  private final HttpClient http = HttpClient.newHttpClient();
  final String url;

  Serve() throws IOException {
    File root = new File(System.getProperty("lingara.root"));
    process =
        new ProcessBuilder(
                new File(root, "target/debug/conformance-server").getPath(),
                "serve",
                "--cases",
                new File(root, "conformance/ek/cases").getPath(),
                "--registry",
                new File(root, "spec/versions.toml").getPath(),
                "--port",
                "0")
            .redirectError(ProcessBuilder.Redirect.INHERIT)
            .start();
    BufferedReader out =
        new BufferedReader(new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8));
    String line;
    String port = null;
    while (port == null && (line = out.readLine()) != null) {
      if (line.startsWith("listening ")) {
        port = line.substring("listening ".length());
      }
    }
    if (port == null) {
      throw new IOException("conformance-server printed no `listening <port>` line");
    }
    url = "http://127.0.0.1:" + port;
  }

  String post(String verb, String caseId) throws IOException, InterruptedException {
    HttpRequest request =
        HttpRequest.newBuilder(URI.create(url + "/__conformance/" + verb + "/" + caseId))
            .header("content-type", "application/json")
            .POST(HttpRequest.BodyPublishers.ofString("{}"))
            .build();
    return http.send(request, HttpResponse.BodyHandlers.ofString()).body();
  }

  @Override
  public void close() {
    process.destroy();
  }
}
