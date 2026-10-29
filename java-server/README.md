# lingara-embed-server-java

> `MemoryPlayerDirectory` is lost on restart and not shared between instances: it is for tests and single-process demos. In production, implement `PlayerDirectory` over your own database ([below](#the-player-directory)).

The official Java server kit for embedding [Lingara](https://getlingara.com) in a game or a site. Your server already holds your Lingara client credentials; vouching for a player is four small jobs, and this kit does each in a call: **mint** a player's token and hand it over, render the **widget** snippet, **route** a verified webhook to the player it concerns, and **delete** a player.

- Java 17 or newer, server-side JVM. A named module, `com.getlingara.embed.server`, that works on the class path too.
- One runtime dependency: the Lingara library, `com.getlingara:lingara-java`. The client, its token cache, retries and errors, `MintedToken` and the webhook verifier are all the library's; Jackson comes through it.
- No web framework. The kit takes the raw body as `byte[]` and the headers as `Map<String, List<String>>`, so servlets, Spring, Javalin and the JDK's `HttpServer` all work with no glue.
- Not the player kit: a game's client uses `lingara-embed-java`. Neither artifact depends on the other.

## Install

The kit is published to Lingara's own Maven repository, `https://maven.getlingara.com`, so your build names it beside the dependency. The versions published before the move stay on Maven Central too; new ones are only here.

Maven:

```xml
<repositories>
  <repository>
    <id>lingara</id>
    <url>https://maven.getlingara.com</url>
  </repository>
</repositories>

<dependency>
  <groupId>com.getlingara</groupId>
  <artifactId>lingara-embed-server-java</artifactId>
  <version>VERSION</version>
</dependency>
```

Gradle:

```kotlin
repositories {
    exclusiveContent {
        forRepository { maven("https://maven.getlingara.com") }
        filter { includeGroup("com.getlingara") }
    }
    mavenCentral()
}

dependencies {
    implementation("com.getlingara:lingara-embed-server-java:VERSION")
}
```

Build one server and one router at startup, from the environment. Only a **metered** client may mint.

```java
LingaraClient client = LingaraClient.builder()
    .clientCredentials(System.getenv("LINGARA_CLIENT_ID"), System.getenv("LINGARA_CLIENT_SECRET"))
    .build();
PlayerDirectory players = new JdbcPlayerDirectory(dataSource);   // yours, below
EmbedServer embed = EmbedServer.of(client, players);
PlayerRouter router = PlayerRouter.of(players, System.getenv("LINGARA_WEBHOOK_SECRET"));
```

## 1. Mint, and hand the token over

Your own route, behind your own login. It mints with this page's origin and returns exactly `{"token":"lgr_et_…","expires_at":"…"}`, the body every player kit and the widget read:

```java
server.createContext("/lingara/token", exchange -> {
  String playerId = yourSession(exchange);                      // your own login
  if (playerId == null) { exchange.sendResponseHeaders(401, -1); return; }
  MintedToken minted = embed.mintToken(playerId, MintOptions.none().withOrigin("https://game.example"));
  byte[] body = TokenHandoff.body(minted).getBytes(StandardCharsets.UTF_8);
  exchange.getResponseHeaders().set("Content-Type", "application/json");
  exchange.getResponseHeaders().set("Cache-Control", "no-store");
  exchange.sendResponseHeaders(200, body.length);
  exchange.getResponseBody().write(body);
  exchange.close();
});
```

- **Every call is one mint.** The kit keeps no cache, on purpose: a cached token could already be revoked by a `deletePlayer` on another instance. **Rate-limit this route per session.**
- `origin` is an argument, never read from the request: pass your page's origin, one of your client's registered embed origins. A game's server mints without one: `embed.mintToken(playerId)`.
- `withScopes(List.of("events:read", "embed:play"))` narrows the token; without it, the token gets every handable scope the client holds.
- On success the kit calls `players.remember(playerId, minted.subject())`. On a failure it remembers nothing and throws the library's own `LingaraException` (`ApiException`, `OAuthException`, `MaintenanceException`, `TransportException`) unchanged.
- `MintedToken` renders as `[REDACTED]`; `TokenHandoff.body` is the one place its token is read.

## 2. The widget snippet

```java
String html = WidgetSnippet.render(
    SnippetOptions.builder(planId, "/lingara/token")   // the plan you made for this player
        .locale("fr")
        .nonce(cspNonce)                                 // this request's CSP nonce, if you set one
        .build());
```

It returns the `<lingara-embed>` element and two module scripts, with no trailing newline. The token never appears in the markup: the element `POST`s to `tokenEndpoint` at run time with **`credentials: "same-origin"`**, so your token route authenticates by the page's own cookie. An absolute `tokenEndpoint` on another origin receives no cookies, and must authenticate some other way.

- **CSP.** On a page whose `script-src` allows no inline script, pass the request's nonce: it is written on both scripts. Without it a strict page blocks the third line silently.
- Options: `surface` (default `practice`), `theme` (`light`, `dark`, `auto`), `locale`, `lingaraOrigin`, `title`, `id` (default `lingara-embed`; two widgets on one page need two ids), `nonce`, `elementSrc` (default the CDN build of this release).
- A value that breaks its rule throws `SnippetException`, whose `field()` names the option (`planId`, `tokenEndpoint`, …). Attribute values and the inline script are escaped by the kit.

## 3. Route a webhook to its player

```java
server.createContext("/lingara/webhook", exchange -> {
  byte[] body = exchange.getRequestBody().readAllBytes();       // raw, before anything parses it
  int status = router.handle(body, exchange.getRequestHeaders(), routed -> {
    if (seenBefore(routed.event().id())) return;                // delivery is at least once
    if (routed.event() instanceof PracticeCompleted done && routed.playerRef().isPresent()) {
      award(routed.playerRef().get(), done);
    }
  });
  exchange.sendResponseHeaders(status, -1);                     // always an empty body
  exchange.close();
});
```

`handle` verifies the signature on the raw body **before** anything else, so a forged event never reaches your directory or your handler, and returns the status to send:

| Outcome | Status |
|---|---|
| your handler returned | `204` |
| a bad signature (missing header, bad timestamp, no matching signature) | `401` |
| a valid signature over a body that is not an event | `400` |
| your handler or your directory threw | `500`; the cause goes to `onError` |

- Every status has an empty body, and Lingara redelivers every non-`2xx`.
- **Deduplicate on `event.id()`**: delivery is at least once.
- `routed.playerRef()` is empty for a subject your directory does not know: your own subject, or a deleted player. An event type newer than the library arrives as `UnknownEvent`; acknowledge it.
- The handler runs inline and Lingara expects a `2xx` within 10 s: enqueue slow work and return.
- `onError` defaults to an error line on `System.getLogger("com.getlingara.embed.server")`; pass your own with `router.withOnError(e -> log.error("lingara webhook", e))`.
- During a secret rotation, pass both: `PlayerRouter.of(players, newSecret, oldSecret)`. A malformed secret fails at startup.
- `router.route(body, headers)` returns the `RoutedEvent` and throws the library's `WebhookVerificationException`, for a route that maps statuses itself.

## 4. Delete a player

```java
embed.deletePlayer(playerId);   // from your account-deletion path
```

It revokes every token the player holds and then calls `players.forget(playerId)`. An unknown player is a success, so a retry is safe; on a failure the row is kept, because the player still exists at Lingara. Deleting your own player data is yours.

## The player directory

`subject` is a pairwise identifier Lingara computes, so no kit can turn it back into your player. The mint's answer is the only moment both are in one place, and the kit records it there through your `PlayerDirectory`:

| Column | Index |
|---|---|
| `player_ref` | plain index |
| `subject` | **unique** index |

```java
public record JdbcPlayerDirectory(DataSource db) implements PlayerDirectory {
  public void remember(String playerRef, String subject) {   // upsert keyed on subject
    update("INSERT INTO lingara_players (player_ref, subject) VALUES (?, ?)"
        + " ON CONFLICT (subject) DO UPDATE SET player_ref = excluded.player_ref", playerRef, subject);
  }
  public Optional<String> playerRefFor(String subject) { /* SELECT player_ref … WHERE subject = ? */ }
  public void forget(String playerRef) {                     // every row for the player
    update("DELETE FROM lingara_players WHERE player_ref = ?", playerRef);
  }
}
```

Unique on `subject` only: a player who links a Lingara account owns a second subject beside the guest's. Whatever your directory throws, the kit propagates unchanged (and `handle` answers `500`). `MemoryPlayerDirectory` is the process-local one, with `snapshot()` for tests.

## The rest

- The snippets the documentation site shows are compiled in CI: [`snippets/server/java/`](../snippets/server/java/).
- Behaviour is held to the server-kit contract, [`conformance/es/CONTRACT.md`](../conformance/es/CONTRACT.md), by the harness in [`conformance-es/`](conformance-es/).
- Nothing the kit renders (its objects, errors or log lines) carries `lgr_cs_`, `lgr_whsec_` or `lgr_et_`.
