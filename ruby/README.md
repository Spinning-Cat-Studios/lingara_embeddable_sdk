# lingara-embed (Ruby)

> `Lingara::Embed::MemoryDirectory` is lost on restart and not shared between
> processes: production code passes a directory over its own database
> ([below](#the-player-directory)).

The Ruby server kit for embedding [Lingara](https://getlingara.com) in a game
or a website. Your server already holds the client secret, so it vouches for
its players. That takes four small jobs, and this gem does each one in a few
lines over the [`lingara`](https://rubygems.org/gems/lingara) library:

1. **mint** a player's embed token;
2. **hand it over** to the player's game or browser;
3. **route** Lingara's signed webhooks to the player each one concerns;
4. **delete** a player.

It also renders the `<lingara-embed>` widget snippet for a web page.

## Install

```sh
gem install lingara-embed
```

or, in a Gemfile, `gem "lingara-embed"`. Ruby 3.3 or newer. The gem's one
runtime dependency is `lingara`. `require "lingara/embed"` loads both.

```ruby
require "lingara/embed"

client = Lingara::Client.new(client_id: ENV.fetch("LINGARA_CLIENT_ID"),
                             client_secret: ENV.fetch("LINGARA_CLIENT_SECRET"))
players = PlayerTable.new(DB)  # your directory, below
server = Lingara::Embed::Server.new(client, directory: players)
router = Lingara::Embed::PlayerRouter.new(ENV.fetch("LINGARA_WEBHOOK_SECRET"), directory: players)
```

Only a **metered** client may mint. Every other client gets
`403 embed_needs_metered`.

## 1. Mint, and hand the token over

The player side asks your server for a token. This is your route, behind
your own sign-in, so it only ever mints for the user it has authenticated.
Here it is in Sinatra:

```ruby
post "/lingara/token" do
  halt 401 unless current_user
  minted = server.mint_token(current_user.id.to_s, origin: "https://game.example")
  content_type :json
  cache_control :no_store
  Lingara::Embed.token_handoff(minted)  # {"token":"lgr_et_…","expires_at":"…Z"}
end
```

- `mint_token(player_ref, scopes: nil, origin: nil)` makes exactly one
  `POST /v1/embed/tokens` and returns the library's `Lingara::MintedToken`.
  It then calls `directory.remember(player_ref, subject)`. Nothing is
  cached. Each token lives 900 s, and the player kit asks again before it
  expires.
- `origin:` is your page's origin. It must be one of your client's
  registered embed origins. A game's server mints without one.
- `token_handoff` returns the one body every player kit and the widget read,
  `{"token","expires_at"}`, with `expires_at` in UTC. It is the only place
  the kit exposes the token. Everywhere else the token renders
  `[REDACTED]`.
- **Rate-limit this route per session.** Lingara limits each player, but
  your route is the door anyone can knock on.
- A failure raises the library's `Lingara::Error` (`ApiError`,
  `OAuthError`, `MaintenanceError`, `TransportError`) unchanged, and nothing
  is remembered. The library does the retries and the token refresh, so the
  kit adds none of its own.

## 2. Route a webhook to its player

```ruby
post "/lingara/webhook" do
  status router.handle(request.body.read, request.env) { |routed|
    next if seen?(routed.event.id)  # delivery is at least once: dedup on event.id
    PracticeJob.perform_later(routed.player_ref, routed.event.to_h) if routed.player_ref
  }
end
```

`handle(body, headers)` verifies the signature on the **raw** body first,
then looks the subject up in your directory, then calls your block with a
`RoutedEvent` (`event`, `player_ref`). It returns the status for your
route, always with an empty body:

| Outcome | Status |
|---|---|
| your block returned | `204` |
| a bad or missing signature, or a stale timestamp | `401`: neither the directory nor your block is called |
| a valid signature over a body that is not an event | `400` |
| your block or your directory raised | `500`: the error goes to `on_error:` |

`headers` is a Hash or a Rack `env`, as it is. `player_ref` is `nil` for a
subject your directory does not know, such as your own or a deleted
player's. An event type newer than the library arrives as
`Lingara::Events::UnknownEvent`. Answer it `204` like any other.

- **Deduplicate on `event.id`.** Lingara redelivers every non-`2xx`, so the
  same event can arrive twice.
- Lingara waits 10 s for an answer. Enqueue slow work and return.
- `on_error:` defaults to `Kernel#warn`. Pass your logger:
  `PlayerRouter.new(secrets, directory: players, on_error: ->(e) { Rails.logger.error(e) })`.
- During a secret rotation, pass both secrets:
  `PlayerRouter.new([new_secret, old_secret], directory: players)`.
- `route(body, headers)` returns the `RoutedEvent`, or raises
  `Lingara::Events::VerificationError`, for a route that maps statuses
  itself.

## 3. Delete a player

```ruby
def destroy_account(user)
  server.delete_player(user.id.to_s)  # revokes their tokens, then forgets their rows
  user.destroy!
end
```

Call this from your account-deletion path. Deleting an unknown player
succeeds, so a retry is safe. If the call fails, the directory keeps its
rows.

## 4. The widget snippet

```erb
<%= Lingara::Embed.widget_snippet(plan_id: plan.id, token_endpoint: "/lingara/token",
                                  locale: "fr", nonce: content_security_policy_nonce).html_safe %>
```

This renders the `<lingara-embed>` element and its two module scripts. The
token is never in the markup. The element `POST`s to `token_endpoint` at
run time, with `credentials: "same-origin"`, and reads `token_handoff`'s
body.

- **The CSP nonce.** If your page's `script-src` allows no inline script,
  pass the request's nonce. Without it, a strict page blocks the third line
  without any error.
- **Same-origin credentials.** The token route is authenticated by your
  page's cookies. An absolute `https://` endpoint on another origin gets no
  cookies, so it must authenticate the user some other way.
- Keywords: `plan_id:` and `token_endpoint:` (required), `surface:`,
  `theme:`, `locale:`, `lingara_origin:`, `title:`, `id:` (one per widget on
  a page), `nonce:`, `element_src:`. An invalid value raises
  `Lingara::Embed::SnippetError`, whose `field` names the option
  (`"planId"`, `"tokenEndpoint"`, …).

## The player directory

`subject` is a pairwise identifier that Lingara computes. Your server cannot
derive a `player_ref` from it, so the mint records the pair, and the router
reads it back. A directory is any object that answers these three methods:

```ruby
class PlayerTable
  def initialize(db) = @rows = db[:lingara_players]

  def remember(player_ref, subject)  # an upsert keyed on subject
    @rows.insert_conflict(target: :subject, update: {player_ref: player_ref})
      .insert(player_ref: player_ref, subject: subject)
  end

  def player_ref_for(subject) = @rows.where(subject: subject).get(:player_ref)

  def forget(player_ref) = @rows.where(player_ref: player_ref).delete  # every row, idempotent
end
```

Store it beside your players, in two columns:

| `player_ref` | `subject` |
|---|---|
| `TEXT NOT NULL`, plain index | `TEXT NOT NULL`, **unique** index |

A player may own more than one subject. A guest who links an account gets
a second one, so the table is unique on `subject` only. The directory is
optional for `Server` and required for `PlayerRouter`.

## Conformance

`make test-ruby` runs the unit tests (the shared snippet, handoff and
webhook vectors), RuboCop and `rbs validate`. `make conformance-es-ruby`
runs every case of the ES contract (`conformance/es/CONTRACT.md`) through
this kit. The receiver runs on rack and webrick from
`conformance-es/Gemfile`, which never reaches the gem.

## License

MIT
