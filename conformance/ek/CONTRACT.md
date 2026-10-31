# EK — the player-side contract

## Scope

Every binding over the core: Rust (`core/`), `bevy/`, `dotnet/` (over
`ffi/`), `godot/`, `unreal/` (through a C++ harness over `ffi/`), `kotlin/`
and `java/`. A player kit is a client of Lingara holding only an embed token.
The conformance server answers, streams and judges what the kit sent.

The web widget is not a binding: its contract is the hosted page's
postMessage `v1`.

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

Ids are `EK<n>`, listed in [`behaviours.toml`](behaviours.toml); the
operations this tier covers are in [`operations.toml`](operations.toml).
Every binding over the core replays every case through its own surface.

- **EK1 — only an embed token crosses.** Every request carries
  `Authorization: Bearer lgr_et_…`. A token from any source that is not an
  embed token is refused locally (`invalid_input`, not retryable) and never
  sent.
- **EK2 — the token is asked for before it goes stale, and once after a
  `401`.** A token is stale at `lifetime − min(60 s, lifetime / 2)`, and the
  kit asks for the next one then (`token_needed`, reason `expiring`). After a
  `401` it asks once more (reason `rejected`) and retries the call once; a
  second `401` is `token_rejected`.
- **EK3 — the stream resumes.** After a drop the stream reopens with
  `Last-Event-ID` set to the last id it read. A gap of
  `stream_idle_timeout_ms` without a byte (45 s by default: three missed
  heartbeats) counts as a drop. Each event id reaches the host once.
- **EK4 — an inbound send is one event.** One `Idempotency-Key`, drawn when
  the call is made, goes on every retry.
- **EK5 — a practice session reports once.** Exactly one
  `world.practice_completed`, `{plan_id, completed, items[{set_number,
  question, outcome}]}`: `set_number` is the plan's own set number,
  `question` the 0-based index within that set, every pair once, at most 100
  items. `completed` is `false` when the session was finished early, and
  every question not graded then is `skipped`. A session started with
  `report: false` (a demo plan) sends nothing.
- **EK6 — a dialogue turn sends a bounded window and maps four frames.**
  `history` holds at most 12 entries, the oldest dropped first, with no
  character total; an NPC reply is stored cut to its first 500 Unicode scalar
  values, and the line before it is stored with it. The frames reach the host
  in order as `opened` (the response headers), `fragment` per `delta`,
  `notice`, and `finished` with the whole reply; `error` ends the turn. No
  `started` frame is expected, and a turn is never re-sent.
- **EK7 — a plan being made is waited for.** A practice request sends
  `world.practice_requested` with `generate: true`. On `plan_status:
  generating` the session waits for `lesson_plan.ready` with that `plan_id`
  on the stream, then reads the plan; `lesson_plan.failed`, and a `refused`
  reaction, end the session in `failed`. Any other status is a plan served
  from the pool, read at once. A `create` session reads `POST
  /v1/lesson-plans` to its `result`, reopening the plan's own stream once if
  the first drops after `started`.
- **EK8 — errors map to one closed code.** Each refusal surfaces as its
  `kind`, with `retryable` a function of the kind alone, and `status`, `code`
  and `retry_after_ms` carried:

  | Answer | `kind` | `retryable` |
  |---|---|---|
  | `400`, `422` | `invalid_input` | no |
  | `401` after the one retry | `token_rejected` | no |
  | `402` | `billing` | no |
  | `403` (`insufficient_scope`, `embed_needs_metered`, …) | `forbidden` | no |
  | `404` | `not_found` | no |
  | `410` | `config` | no |
  | `429` | `rate_limited`, with `retry_after_ms` | yes |
  | `503` without a JSON body, `5xx`, an `error` frame in a `200` stream (`status: 200`) | `unavailable` | yes |
  | a dropped connection | `network` | yes |

- **EK9 — close and backpressure.** After `close` the server sees every
  connection drop within 2 s, and a close-capable harness sees one `closed`
  and nothing after it. An ABI harness that stops with `free` sees no
  `closed`, and is held to the server's 2 s check alone. A host that stops
  polling for longer than the queue holds loses no stream event.
- **EK10 — the kit names itself.** The `User-Agent` is the library's own
  token, then `lingara-embed/<version>`, then at most one kit token (a
  binding's, such as `lingara-godot/<version>`), then the host's suffix.
  `Lingara-Version` is sent when the kit is pinned.
- **EK11 — a graph dialogue sends only passing edges, in order, and crosses
  exactly one** (ADR 28.10.26f). A turn on a `dialogue_load`ed graph carries
  its `checkpoint`, the open edges that have a `when` in IR order (never a
  `predicate`), and values only for keys in the checkpoint's `may_disclose`.
  An edge with no `when` is crossed with no turn (`checkpoint_reached`,
  `via: auto`). A `transition` is applied after `done`, and only for an edge
  the turn sent (`via: judge`); an error after it, or a
  `transition_unavailable` notice, leaves the checkpoint. An edge to an
  outcome ends the dialogue (`dialogue_ended`), and a further line is
  `invalid_state`.
- **EK12 — a stored dialogue is fetched with the embed token, at most once
  per id per freshness window** (ADR 28.10.26i). A `dialogue_load` with
  `source: { type: stored, dialogue_id }` sends `exportDialogue` with the
  embed bearer and no `revision` query, and answers `dialogue_loaded` with
  the `Lingara-Dialogue-Revision` header's `rev` before `checkpoint_reached`
  (`via: start`). A second load of the same id within 300 s sends nothing.
  `404 not_found` and `404 dialogue_not_active` fail the load
  (`dialogue_load_failed`) as `not_found` with no retry, a `403` as
  `forbidden`, and a body the IR's validation refuses as `invalid_input`.
- **EK13 — an update notice is said once per handle** (ADR 30.10.26p). The
  first token carrying `update_available` yields one `log` at `warn`
  (target `lingara_embed::update`, the line
  `conformance/vectors/update-notice.json` pins) and one
  `update_available`, in that order; a later token yields neither; a token
  without it yields neither; and the token is accepted either way. A
  malformed `update_available` is dropped, never refused. Nothing in it is
  sent to Lingara.

## The host script

A case's `host:` steps are written in the core's JSON vocabulary, which every
binding already maps. Every step and field is closed.

| Step | Fields | Through the surface |
|---|---|---|
| `start` | `initial_token`: `none` or a token; `config`: a subset of `EmbedConfig` (`queue_capacity`, `api_version`, `user_agent_suffix`, `event_types`, `resume_cursor`, `stream_on_start`, `stream_idle_timeout_ms`, `log_level`). `base_url` is always the server's | `start`, then `provide_token_for(token, 900)` if a token is given |
| `provide_token` | `token`, and exactly one of `expires_at` or `expires_in_secs`; optionally `update_available`, passed through as written, malformed or not (EK13) | `provide_token` or `provide_token_for`, the binding's way of carrying `update_available` when it is set |
| `command` | a `Command` in its JSON shape. A value `"$last"` is the id the latest `practice_start`, `dialogue_open` or `dialogue_load` answered | `execute`, or the binding's flat method |
| `poll_until` | `event`: an event `type` tag (`stream`, `npc_turn`, …); `match`: an optional object the event must contain, compared after dropping nulls; `within_ms` | drain until a match, failing at `within_ms` |
| `expect_no_event` | `event`, optional `match`, `for_ms` | drain for `for_ms`; fail on a match |
| `stop_polling` | `for_ms` | do not drain for `for_ms` |
| `close` | none | `close`, then drain to `closed`; an ABI harness frees instead |

- **A refusal is an event.** A `provide_token` or `command` the core
  refuses is seen by the harness as `{"type": "error", "error": …}`, before
  anything still queued, so a case asserts a refusal with `poll_until`.
- **`match` contains.** An object matches when each of its non-null keys
  does; an array when it has the same length and each element matches;
  anything else when equal.
- `expect.events`, when a case sets it, is the list of events the
  `poll_until` steps matched, in order, JSON-equal after dropping nulls.
- **Timing-free.** No case asserts a backoff delay, and a case waits at most
  a few seconds of real time: a stale point is tested with a 2 s lifetime,
  a heartbeat gap with `stream_idle_timeout_ms: 1000`.

Besides R1's exact `headers`, a step's `expect_request` may set `repeat` (a
header equal to the previous request's: EK4's key), `body` (an object the
request body contains: EK5's report, EK6's window) and `user_agent_tail`
(EK10's chain).

## Harness rules

- A harness drives its kit through the kit's public surface only.
- It reports every case it was given. **There is no `skip`.**
- A binding whose engine cannot run in public CI runs its harness over the
  layer beneath it, and names that layer in its own docs.

See [`../README.md`](../README.md) for the case format and how to run the
server.
