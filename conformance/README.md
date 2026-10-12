# Conformance

Every embed kit is held to one of two contracts, and the conformance server
plays Lingara for both:

| Tier | Contract | The kit is | The server |
|---|---|---|---|
| **EK** | [`ek/CONTRACT.md`](ek/CONTRACT.md) | a player kit, holding an embed token | answers, streams and judges what the kit sent |
| **ES** | [`es/CONTRACT.md`](es/CONTRACT.md) | a server kit, holding the client secret | answers the token grant, mint and delete, and posts signed webhooks to the kit's receiver |

If you port the core to a language we don't ship, these cases and this
server are how you know your port behaves.

## Layout

```
conformance/
├── vectors/            webhook-signatures.json + SOURCE (the release it was copied from, and its sha256)
├── ek/  es/            CONTRACT.md · behaviours.toml · operations.toml · cases/<group>/<slug>.yaml
└── server/             the conformance server (Rust)
```

`behaviours.toml` is each tier's closed list of behaviour ids;
`operations.toml` is its closed list of API operations (`operationId`s) and
messages. The server reads the frozen API version in `spec/versions/`, never
the live `spec/openapi.json`.

## The case format

One file is one case, `<tier>/cases/<group>/<slug>.yaml`, with id
`<group>.<slug>`. Every key is closed: an unknown one is refused.

```yaml
id: ek3.resume-after-drop
behaviours: [EK3]
tier: ek
serve:                                   # the server's scripted answers, in order
  - route: { operation: streamEvents }   # an operations.toml entry, or { token_grant: {} }
    respond:
      sse:
        - { id: "e1", event: "lesson_plan.ready", data: { plan_id: "…" } }
        - { comment: "heartbeat" }
      fault: { drop_after_events: 1 }    # or heartbeat_gap_s · delay_fragment_ms
  - route: { operation: streamEvents }
    expect_request: { headers: { last-event-id: "e1" } }
    respond: { status: 401, json: { error: "unauthorized" } }
host:                                    # EK: what the harness does through its kit, in order
  - start: { initial_token: none }
  - provide_token: { token: "lgr_et_conformance0001", expires_in_secs: 900 }
  - poll_until: { event: stream, within_ms: 2000 }
  - close: {}
expect:
  events: [ … ]                          # what the host saw, JSON-equal after dropping nulls
  requests: { count: 2 }
```

- **`respond`** is `json` or `sse`, with an optional `status` and `headers`.
  An SSE frame is an event (`data`, optional `id` and `event`) or a
  `comment` (a heartbeat). A mid-session `401` is just a step whose
  `respond` has that status.
- **Faults:** `drop_after_events` aborts the connection after N events;
  `heartbeat_gap_s` holds the stream open and silent that long after its
  frames; `delay_fragment_ms` writes each frame in two writes.
- **`host:`** steps are commands to the harness: `start`, `provide_token`,
  `command`, `poll_until`, `expect_no_event`, `stop_polling`, `close`. Each
  binding maps them onto its surface; their fields are in
  [`ek/CONTRACT.md`](ek/CONTRACT.md), *The host script*.
- **`expect_request`** holds exact `headers`, and may add `repeat` (headers
  equal to the previous request's), `body` (an object the request body
  contains, after dropping nulls) and `user_agent_tail`.
- **ES cases** have `deliver:` instead of `host:`:

  ```yaml
  deliver:
    - message: practice.completed
      body: { subject: "lgr_sub_…", data: { … } }
      sign: { secrets: [primary, secondary], timestamp_offset_s: 0, tamper: false }
      expect: { status: 204, handler: { player_ref: p1 } }   # or handler: not_called
  ```

- **ES keys** (ADR 1.10.26af D11), beside `deliver:`:

  ```yaml
  directory: { seed: { lgr_sub_p1: p1 } }   # the harness's MemoryPlayerDirectory before the case
  serve:
    - route: { operation: deleteEmbedPlayer }
      expect_request: { path: "/v1/embed/players/p%2F1", body_equals: {} }   # exact raw path; exact body
      respond: { status: 204 }
  kit:                                      # the jobs the harness performs through its kit, in order
    - mint: { player_ref: p1, scopes: [events:read], origin: "https://game.example" }
    - delete: { player_ref: p1 }
    - handoff: {}                           # tokenHandoff over the last minted token
    - render: {}                            # ES5's sweep, after the last delivery
  deliver:
    - message: practice.completed
      as_type: world.weather_changed        # optional: a type newer than the libraries
      body: { subject: "lgr_sub_p1", … }
      sign: { secrets: [primary] }
      handler: raise                        # ok (default) | raise
      router: { secrets: [secondary] }      # the PlayerRouter for this delivery; default [primary]
      expect: { status: 500, handler: { player_ref: p1, event_id: "lgr_evt_…", unknown: false } }
  expect:
    directory: { }                          # subject → player_ref after the case
    directory_lookups: 0                    # playerRefFor calls the harness counted
    minted: { token, expires_at, expires_in, subject, scopes, account_linked }
    handoff: '{"token":"…","expires_at":"…"}'
    error: { status: 403, code: embed_needs_metered }   # code `maintenance` for a non-JSON 503
  ```

  The server judges what it sees (each request, `expect.requests`, each
  delivery's status, and each reported handler's `player_ref`); the harness
  judges the rest, as [`es/CONTRACT.md`](es/CONTRACT.md)'s *What a harness
  reports* sets out.

## Vectors

`conformance/vectors/` holds what no case can carry, read directly by each
kit's unit tests from the repository root:

| File | Shape | Held by |
|---|---|---|
| `webhook-signatures.json` | E7's file, copied at the tag in `SOURCE` | each verifier (C#'s own) |
| `snippet.json` | `{description, cases: [{name, options, expect: {html} \| {refused: <field>}}]}`; `{{version}}` in `html` is the kit's `VERSION` | ES4, every server kit's `widgetSnippet` |
| `token-handoff.json` | `[{name, input: {token, expires_at}, body}]`, `body` the exact text | ES6, every server kit's `handoffBody`; K1's reader |
| `json-shape.json` | the core's message shapes | EK bindings |

## What the server judges

On every request: the method and path against the step's route, a JSON body
against the frozen schema for its `operationId`, and the step's
`expect_request` headers. Whatever a case says:

- **EK:** every request carries `Authorization: Bearer lgr_et_…`, and no
  `lgr_at_`, `lgr_cs_` or `client_secret` appears in any header or body.
  At finish, every stream the case opened must have been dropped within 2 s
  (EK9): a harness finishes a case after closing its kit.
- **ES:** a mint or delete carries `Bearer lgr_at_…`, the token the case's
  grant issued; a token grant's `grant_type` is `client_credentials`.

## Harness rules

- A harness drives its kit through the kit's **public surface only**.
- It reports **every case** it was given. There is no `skip`.
- A binding whose engine cannot run in public CI runs its harness over the
  layer beneath it, and names that layer.

## Running the server

```sh
# one binding against every case of its tier
conformance-server run --tier ek --lang rust -- <harness command…>

# cases with no harness (a demo, or debugging); prints `listening <port>`
conformance-server serve --cases <dir> --port 0

# the case inventory against the tier's lists and the frozen API version
conformance-server check-coverage --tier ek --registry spec/versions.toml --langs "rust"
```

`run` starts the harness with `LINGARA_EMBED_SERVER_URL`,
`LINGARA_EMBED_RESULTS` and `LINGARA_EMBED_TIER`. An ES harness also gets
`LINGARA_EMBED_WEBHOOK_SECRETS`, `LINGARA_EMBED_CLIENT_ID` and
`LINGARA_EMBED_CLIENT_SECRET` (all visibly fake), and must print
`listening <port>` for its webhook receiver within 30 seconds.

For each case the harness calls `GET /__conformance/cases/{id}`,
`POST /__conformance/arm/{id}`, drives its kit, then
`POST /__conformance/finish/{id}` (an ES harness sends
`{"handlers": [<player_ref or null>, …]}`). Finish answers the verdict:
`{"case", "result": "pass" | "fail", "mismatches": […]}`.

The results file is one JSON object per line,
`{"case", "lang", "result", "mismatches"}`. The run fails when it omits an
armed case, reports a case never armed, or holds anything but `pass`.
`--only <ids>` is for local debugging and is refused when `CI` is set.
