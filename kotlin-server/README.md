# lingara-embed-server-kotlin

> `MemoryPlayerDirectory` is lost on restart and not shared between instances: it is for tests and single-process demos. In production, implement `PlayerDirectory` over your own database ([below](#the-player-directory)).

The official Kotlin server kit for embedding [Lingara](https://getlingara.com) in a game or a site. Your server already holds your Lingara client credentials; vouching for a player is four small jobs, and this kit does each in a call: **mint** a player's token and hand it over, render the **widget** snippet, **route** a verified webhook to the player it concerns, and **delete** a player.

- Kotlin on JVM 17 or newer. The jobs are `suspend` functions, and so are the directory's.
- One runtime dependency: the Lingara library, `com.getlingara:lingara-kotlin`. The client, its token cache, retries and errors, `MintedToken` and the webhook verifier are all the library's; coroutines and kotlinx-serialization come through it.
- No web framework. The kit takes the raw body as `ByteArray` and the headers as `Map<String, List<String>>`, so Ktor, Spring and http4k all work with no glue.
- Not the player kit: a game's client uses `lingara-embed-kotlin`. Neither artifact depends on the other.

## Install

```kotlin
implementation("com.getlingara:lingara-embed-server-kotlin:VERSION")
```

Build one server and one router at startup, from the environment. Only a **metered** client may mint.

```kotlin
val client = LingaraClient {
    clientCredentials(System.getenv("LINGARA_CLIENT_ID"), System.getenv("LINGARA_CLIENT_SECRET"))
}
val players = JdbcPlayerDirectory(dataSource)                    // yours, below
val embed = EmbedServer(client, players)
val router = PlayerRouter(players, System.getenv("LINGARA_WEBHOOK_SECRET"))
```

## 1. Mint, and hand the token over

Your own route, behind your own login. It mints with this page's origin and returns exactly `{"token":"lgr_et_…","expires_at":"…"}`, the body every player kit and the widget read:

```kotlin
post("/lingara/token") {
    val playerId = call.yourSession() ?: return@post call.respond(HttpStatusCode.Unauthorized)
    val minted = embed.mintToken(playerId, origin = "https://game.example")
    call.response.header(HttpHeaders.CacheControl, "no-store")
    call.respondText(tokenHandoff(minted), ContentType.Application.Json)
}
```

- **Every call is one mint.** The kit keeps no cache, on purpose: a cached token could already be revoked by a `deletePlayer` on another instance. **Rate-limit this route per session.**
- `origin` is an argument, never read from the request: pass your page's origin, one of your client's registered embed origins. A game's server mints without one: `embed.mintToken(playerId)`.
- `scopes = listOf("events:read", "embed:play")` narrows the token; without it, the token gets every handable scope the client holds.
- On success the kit calls `players.remember(playerId, minted.subject)`. On a failure it remembers nothing and throws the library's own `LingaraException` (`ApiException`, `OAuthException`, `MaintenanceException`, `TransportException`) unchanged.
- `MintedToken` renders as `[REDACTED]`; `tokenHandoff` is the one place its token is read.

## 2. The widget snippet

```kotlin
val html = widgetSnippet(
    SnippetOptions(planId, tokenEndpoint = "/lingara/token").apply {   // the plan you made for this player
        locale = "fr"
        nonce = cspNonce                                                // this request's CSP nonce, if you set one
    },
)
```

It returns the `<lingara-embed>` element and two module scripts, with no trailing newline. The token never appears in the markup: the element `POST`s to `tokenEndpoint` at run time with **`credentials: "same-origin"`**, so your token route authenticates by the page's own cookie. An absolute `tokenEndpoint` on another origin receives no cookies, and must authenticate some other way.

- **CSP.** On a page whose `script-src` allows no inline script, pass the request's nonce: it is written on both scripts. Without it a strict page blocks the third line silently.
- Options: `surface` (default `practice`), `theme` (`light`, `dark`, `auto`), `locale`, `lingaraOrigin`, `title`, `id` (default `lingara-embed`; two widgets on one page need two ids), `nonce`, `elementSrc` (default the CDN build of this release).
- A value that breaks its rule throws `SnippetException`, whose `field` names the option (`planId`, `tokenEndpoint`, …). Attribute values and the inline script are escaped by the kit.

## 3. Route a webhook to its player

```kotlin
post("/lingara/webhook") {
    val headers = call.request.headers.entries().associate { it.key to it.value }
    val status = router.handle(call.receive<ByteArray>(), headers) { routed ->   // raw bytes
        if (seenBefore(routed.event.id)) return@handle                          // at least once
        val event = routed.event
        val playerId = routed.playerRef
        if (event is PracticeCompleted && playerId != null) award(playerId, event)
    }
    call.respond(HttpStatusCode.fromValue(status))                              // always empty
}
```

`handle` verifies the signature on the raw body **before** anything else, so a forged event never reaches your directory or your handler, and returns the status to send:

| Outcome | Status |
|---|---|
| your handler returned | `204` |
| a bad signature (missing header, bad timestamp, no matching signature) | `401` |
| a valid signature over a body that is not an event | `400` |
| your handler or your directory threw | `500`; the cause goes to `onError` |

- Every status has an empty body, and Lingara redelivers every non-`2xx`.
- **Deduplicate on `event.id`**: delivery is at least once.
- `routed.playerRef` is null for a subject your directory does not know: your own subject, or a deleted player. An event type newer than the library arrives as `UnknownEvent`; acknowledge it.
- The handler runs inline and Lingara expects a `2xx` within 10 s: enqueue slow work and return. Cancellation is never swallowed.
- `onError` defaults to an error line on `System.getLogger("com.getlingara.embed.server.kotlin")`; pass your own: `PlayerRouter(players, secret, onError = { log.error("lingara webhook", it) })`.
- During a secret rotation, pass both: `PlayerRouter(players, newSecret, oldSecret)`. A malformed secret fails at startup.
- `router.route(body, headers)` returns the `RoutedEvent` and throws the library's `WebhookVerificationException`, for a route that maps statuses itself.

## 4. Delete a player

```kotlin
embed.deletePlayer(playerId)   // from your account-deletion path
```

It revokes every token the player holds and then calls `players.forget(playerId)`. An unknown player is a success, so a retry is safe; on a failure the row is kept, because the player still exists at Lingara. Deleting your own player data is yours.

## The player directory

`subject` is a pairwise identifier Lingara computes, so no kit can turn it back into your player. The mint's answer is the only moment both are in one place, and the kit records it there through your `PlayerDirectory`:

| Column | Index |
|---|---|
| `player_ref` | plain index |
| `subject` | **unique** index |

```kotlin
class JdbcPlayerDirectory(private val db: DataSource) : PlayerDirectory {
    override suspend fun remember(playerRef: String, subject: String) =   // upsert keyed on subject
        update("INSERT INTO lingara_players (player_ref, subject) VALUES (?, ?) " +
            "ON CONFLICT (subject) DO UPDATE SET player_ref = excluded.player_ref", playerRef, subject)
    override suspend fun playerRefFor(subject: String): String? = TODO("SELECT player_ref … WHERE subject = ?")
    override suspend fun forget(playerRef: String) =                      // every row for the player
        update("DELETE FROM lingara_players WHERE player_ref = ?", playerRef)
}
```

Unique on `subject` only: a player who links a Lingara account owns a second subject beside the guest's. Whatever your directory throws, the kit propagates unchanged (and `handle` answers `500`). `MemoryPlayerDirectory` is the process-local one, with `snapshot()` for tests.

## The rest

- The snippets the documentation site shows are compiled in CI: [`snippets/server/kotlin/`](../snippets/server/kotlin/), the full JDBC directory included.
- Behaviour is held to the server-kit contract, [`conformance/es/CONTRACT.md`](../conformance/es/CONTRACT.md), by the harness in [`conformance-es/`](conformance-es/).
- Nothing the kit renders (its objects, errors or log lines) carries `lgr_cs_`, `lgr_whsec_` or `lgr_et_`.
