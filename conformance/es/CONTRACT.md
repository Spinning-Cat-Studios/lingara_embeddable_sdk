# ES — the server-side contract

## Scope

Every server kit: `core/` with its `server` feature, `typescript/`'s
`/server` subpath, `go/`, `java-server/`, `kotlin-server/`, `ruby/`, `php/`,
and `Lingara.Embed.Server` in `dotnet/`. A server kit is a client of Lingara
holding the client secret (mint, delete), **and** a receiver of signed
webhooks (routing to the right player).

## Delivery

For a case with `deliver:`, the conformance server posts each item to the
harness's webhook receiver, which the harness announces on stdout as
`listening <port>` within 30 seconds. Each post is signed with Standard
Webhooks headers (`webhook-id`, `webhook-timestamp`, `webhook-signature`,
one `v1,` entry per secret, newest first). The receiver is started with the
two secrets it accepts in `LINGARA_EMBED_WEBHOOK_SECRETS`; a delivery signed
with the third, `unknown`, must be refused. At finish the harness reports the
player each delivery's handler saw, `null` for a handler never called.

## The wire

These facts are Lingara's, fixed by the API version the kits pin. A
behaviour below never re-decides one.

- **F1 — the mint.** `POST /v1/embed/tokens` (`createEmbedToken`), body
  `{player_ref (UTF-8, 1–128 bytes), scopes?, origin?}`, under a
  client-credentials `lgr_at_` token holding `embed:mint`, which only a
  metered client holds. It answers 200
  `{token, expires_at, expires_in, subject, scopes, account_linked}` with
  `Cache-Control: no-store`. The token is `lgr_et_` + 43 base62 characters,
  lives 900 s, and is never refreshed by Lingara.
- **F2 — delete.** `DELETE /v1/embed/players/{player_ref}`
  (`deleteEmbedPlayer`), always 204 and idempotent. `player_ref` is
  percent-encoded in the path and decoded by Lingara.
- **F3 — handable scopes.** A token carries a subset of `events:read`,
  `events:write`, `lesson_plans:read`, `lesson_plans:write` and `embed:play`.
  `embed:mint` is client-only and never handed down.
- **F4 — a token's doors.** `GET /v1/events`, `GET /v1/events/stream` (SSE,
  `Last-Event-ID` resume, a heartbeat comment every 15 s), `POST /v1/events`
  and the lesson-plan routes, each filtered to the token's player. Inbound
  `world.practice_completed` `{plan_id, items[{set_number, question,
  outcome}] (1–100), completed?}` needs `events:write` and `embed:play`, is
  refused on a demo plan, and is echoed once as outbound `practice.completed`.
- **F5 — the NPC turn.** `POST /v1/embed/dialogue/turns`
  (`sendDialogueTurn`), stateless. Request `npc{name, persona?}`, `scene?`,
  `source_lang`, `target_lang`, `level` (1–9), `line` (≤ 500), `history`
  (≤ 12 entries of `{speaker: player|npc, text}`). SSE frames `delta{text}`,
  `notice{code,message}`, `done{}`, `error{code,message}`: no `started`, no
  `id:`, not resumable. A non-metered payer gets `403 embed_needs_metered`.
- **F6 — the webhook a server kit routes.** Standard Webhooks signing with
  `lgr_whsec_` secrets; the subject is the pairwise `lgr_sub_` the mint
  returned; the embed message is `practice.completed`.

## Behaviours

Seven behaviours, listed in [`behaviours.toml`](behaviours.toml). ES1–ES5 are
the four jobs and the secret rule; ES6 and ES7 close the two gaps the jobs
leave, the handoff body and the transport under each kit. The vocabulary is
every kit's, spelled in its language's idiom: `EmbedServer` (`mintToken`,
`deletePlayer`), `PlayerRouter` (`route`, `handle`), `PlayerDirectory`
(`remember`, `playerRefFor`, `forget`) with `MemoryPlayerDirectory`, and the
pure `widgetSnippet` and `tokenHandoff`.

- **ES1 — Mint.** `mintToken(playerRef, {scopes?, origin?})` gets a
  client-credentials token holding `embed:mint` and makes exactly one
  `POST /v1/embed/tokens` (F1) per call: the kit keeps **no cache**. `scopes`
  and `origin` are sent only when given; `origin` is an argument, never read
  from a request. On success, and only then, it calls
  `directory.remember(playerRef, subject)` when a directory is configured,
  and returns the library's `MintedToken`. A failure raises the library's
  error unchanged and remembers nothing.
- **ES2 — Delete.** `deletePlayer(playerRef)` makes one
  `DELETE /v1/embed/players/{player_ref}` with `player_ref` as one
  percent-encoded segment (F2). After a `2xx`, and only then, it calls
  `directory.forget(playerRef)`, which removes every subject the player owns.
  A failure keeps the row.
- **ES3 — Verify, then route.** `PlayerRouter` builds the library's verifier
  from its secrets once, at construction. `route(body, headers)` verifies the
  **raw** body (F6), then calls `directory.playerRefFor(event.subject)`, and
  returns `RoutedEvent {event, playerRef}`; `playerRef` is none for a subject
  the directory does not know. An event newer than the library (`UnknownEvent`)
  is routed like any other. `handle(body, headers, onEvent)` answers:

  | Outcome | Status |
  |---|---|
  | `onEvent` returned | `204` |
  | one of the five signature reasons | `401`; neither the directory nor `onEvent` is called |
  | a valid signature over `malformed_payload` | `400` |
  | `onEvent` or the directory raised | `500`; the error goes to `onError` only |

  Every status has an empty body.
- **ES4 — The snippet.** `widgetSnippet(options)` renders the
  `<lingara-embed>` element and its two module scripts exactly as
  [`../vectors/snippet.json`](../vectors/snippet.json) holds them. Attribute
  values escape `& < > " '` as `&amp; &lt; &gt; &quot; &#39;` and nothing
  else; the inline script writes `id` and `tokenEndpoint` as JSON string
  literals escaping `" \` as `\" \\` and `< > & U+2028 U+2029` as `\u003c
  \u003e \u0026 \u2028 \u2029`, with no other escape. The option rules:

  | Option | Rule |
  |---|---|
  | `planId` (required) | `^[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}$` |
  | `surface` (default `practice`) | `^[a-z][a-z0-9-]{0,31}$` |
  | `theme` | `light`, `dark` or `auto` |
  | `locale` | `^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}$` |
  | `lingaraOrigin` | `^(https://[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*(:[0-9]{1,5})?\|http://(localhost\|127\.0\.0\.1)(:[0-9]{1,5})?)/?$` |
  | `title` | 1–80 Unicode scalar values, no control character |
  | `id` (default `lingara-embed`) | `^[A-Za-z][A-Za-z0-9_-]{0,63}$` |
  | `tokenEndpoint` (required) | starts `/` but not `//`, or `https://`; at most 2048 UTF-8 bytes; no control character or whitespace |
  | `nonce` | `^[A-Za-z0-9+/=_-]{8,128}$`, written on both scripts |
  | `elementSrc` (default the CDN build at this `VERSION`) | starts `https://`; at most 2048 bytes; no control character or whitespace |

  A pattern matches ASCII only. A **control character** is Unicode category
  Cc (U+0000–U+001F, U+007F–U+009F); **whitespace** is U+0020, U+0085,
  U+00A0, U+1680, U+2000–U+200A, U+2028, U+2029, U+202F, U+205F and U+3000.
  The first refused option, in the table's order, is raised as
  `SnippetError {field}`, `field` the camelCase name above.
- **ES5 — No secret surfaces.** No rendering of a kit object (server,
  router, directory, minted token), no error and no log line carries
  `lgr_cs_`, `lgr_whsec_` or `lgr_et_`. `tokenHandoff`'s output is the one
  place the token appears, on purpose.
- **ES6 — The handoff.** `tokenHandoff(minted)` returns exactly
  `{"token":"lgr_et_…","expires_at":"…"}`: compact, these two keys in this
  order, `/` unescaped, no HTML escaping. A library that keeps `expires_at` as
  the server's string copies it byte for byte; one that holds a parsed
  instant writes RFC 3339 in UTC with `Z`, fractional seconds only when
  non-zero (trailing zeros trimmed, at most nine digits). The pure
  `handoffBody(token, expiresAt)` beneath it is held to
  [`../vectors/token-handoff.json`](../vectors/token-handoff.json)
  (`[{name, input: {token, expires_at}, body}]`, `body` the exact text), and
  the path from a real mint to `es6.handoff-from-a-mint`.
- **ES7 — The transport under the kit.** A library error reaches the caller
  unchanged (variant, status, code). The contract-K4 retry (`429`/`503` with
  `Retry-After` ≤ 60 s) and the one `401` re-exchange happen once, in the
  library, never again in the kit; the client-credentials token is reused
  across calls. `Lingara.Embed.Server`, which has no library beneath it,
  implements that subset itself and is held to the same cases.

### What a harness reports

`run` serves the cases; the harness drives its kit and judges what only the
kit's caller sees, against the case from `GET /__conformance/cases/{id}`.
Before arming a case it builds a fresh library client (so no token outlives
its case), a `MemoryPlayerDirectory` seeded from `directory.seed` and wrapped
in a decorator counting `playerRefFor` calls, and an in-memory sink that each
router's `onError` and the language's default log facility write to. Then:

1. `POST /__conformance/arm/{id}`; the server starts posting `deliver:`
   items to the receiver. Delivery `n` is answered through a `PlayerRouter`
   built from `deliver[n].router.secrets` (absent: `[primary]`; `primary`
   and `secondary` are the two values of `LINGARA_EMBED_WEBHOOK_SECRETS`, in
   that order), whose `onEvent` raises when `deliver[n].handler` is `raise`.
2. It runs the `kit:` jobs in order: `mint`, `delete`, `handoff` (over the
   last minted token) and `render`. A raised error is recorded as
   `{status, code}` (a maintenance error's code is `maintenance`, an OAuth
   error's its `error`) and the next job runs.
3. It waits until every delivery has been answered, then compares, each
   JSON-equal after dropping nulls: `expect.minted`, `expect.handoff`,
   `expect.error` (a raised error with no `expect.error` is a mismatch),
   `expect.directory` (subject → player_ref), `expect.directory_lookups`,
   and each delivery's `expect.handler` against what its `onEvent` saw
   (`player_ref`, `event_id`, and `unknown` for an `UnknownEvent`). A
   `render` job sweeps the renderings of the server, routers, directory and
   last minted token, and the sink, for the three prefixes — after the last
   delivery.
4. `POST /__conformance/finish/{id}` with `{"handlers": [player_ref | null, …]}`
   in delivery order (`null` for a handler never called or called with no
   player), and writes one result line merging the server's mismatches with
   its own (`fail render` for a leak).

## Harness rules

- A harness drives its kit through the kit's public surface only.
- It reports every case it was given. **There is no `skip`.**
- A binding whose engine cannot run in public CI runs its harness over the
  layer beneath it, and names that layer in its own docs.

See [`../README.md`](../README.md) for the case format and how to run the
server.
