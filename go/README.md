# lingaraembed — Lingara's embed kit for Go servers

Your game or site already has a server, and that server already holds your
Lingara client credentials. Vouching for a player is four small jobs, and this
package does each in a call: **mint** a player's token and hand it over,
render the **widget** snippet, **route** a verified webhook to the player it
concerns, and **delete** a player.

The calls, the token cache, the retries, the errors and the webhook verifier
are all the [`lingara`](https://pkg.go.dev/github.com/Spinning-Cat-Studios/lingara_api_clients/go)
library's. That library is this module's only dependency.

## Install

```sh
go get github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/go
```

Go 1.23 or newer. The package name is `lingaraembed`; import the library
beside it as `lingara`.

```go
import (
	lingara "github.com/Spinning-Cat-Studios/lingara_api_clients/go"
	lingaraembed "github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/go"
)

client, err := lingara.New(lingara.WithClientCredentials(os.Getenv("LINGARA_CLIENT_ID"), os.Getenv("LINGARA_CLIENT_SECRET")))
if err != nil { … }
players := yourDirectory{db} // see "The player directory"
embed := lingaraembed.NewServer(client, lingaraembed.WithDirectory(players))
```

Only a **metered** client may mint (`403 embed_needs_metered` otherwise).

## Mint, and hand the token over

Your own route, behind your own session. The widget calls it with the page's
cookies; a game calls it however it talks to your server.

```go
http.HandleFunc("POST /lingara/token", func(w http.ResponseWriter, r *http.Request) {
	playerRef, ok := currentPlayer(r) // your user, never a value from the request
	if !ok {
		http.Error(w, "sign in first", http.StatusUnauthorized)
		return
	}
	minted, err := embed.MintToken(r.Context(), playerRef, lingaraembed.MintOptions{Origin: "https://game.example"})
	if err != nil {
		http.Error(w, "no token", http.StatusBadGateway)
		return
	}
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Cache-Control", "no-store")
	w.Write(lingaraembed.TokenHandoff(minted)) // {"token":"lgr_et_…","expires_at":"…"}
})
```

- `MintToken` is one `POST /v1/embed/tokens` every time: the kit keeps **no
  cache**, so a token you revoked on one instance is never handed out by
  another. **Rate-limit this route per session yourself**; Lingara's own
  per-player limits cap your client, not your users.
- `Origin` is your page's origin, one of your client's registered embed
  origins. The kit never reads it from a request. A game's server leaves it
  empty. `Scopes` narrows the token; nil asks for every handable scope.
- On success the kit calls `Remember(playerRef, minted.Subject)` on your
  directory. On failure it remembers nothing and returns the library's own
  error (`*lingara.APIError`, `*lingara.OAuthError`, …) unchanged.
- `TokenHandoff` writes exactly `{"token", "expires_at"}`, the body every
  player kit and the widget read. It is the one place the token is exposed;
  everywhere else a `*lingara.MintedToken` renders as `[REDACTED]`.

## The widget snippet

```go
theme := "auto"
html, err := lingaraembed.WidgetSnippet(lingaraembed.SnippetOptions{
	PlanID:        planID,           // the plan your server made for this player
	TokenEndpoint: "/lingara/token", // the route above
	Theme:         &theme,
	Nonce:         &cspNonce,        // your request's CSP nonce
})
```

Three lines: the `<lingara-embed>` element, its module script (pinned to this
kit's release), and an inline module that points the element's
`tokenProvider` at your endpoint. The token itself never appears in the
markup. A refused option is a `*lingaraembed.SnippetError` naming it.

- **CSP.** If your `script-src` allows no inline script, pass the request's
  nonce: without it a strict page silently blocks the third line.
- **Same-origin credentials.** The element fetches your endpoint with
  `credentials: "same-origin"`, so your session cookie goes with it. An
  absolute `https://` endpoint on another origin receives no cookies and must
  authenticate the player some other way; the kit never widens this to
  `include`.

## Route a webhook to its player

```go
router, err := lingaraembed.NewPlayerRouter(players, os.Getenv("LINGARA_WEBHOOK_SECRET"))
if err != nil { … } // a malformed lgr_whsec_… fails here, at startup

http.HandleFunc("POST /lingara/webhook", func(w http.ResponseWriter, r *http.Request) {
	body, _ := io.ReadAll(r.Body) // the raw body: verify before anything parses it
	w.WriteHeader(router.Handle(r.Context(), body, r.Header, func(ctx context.Context, e lingaraembed.RoutedEvent) error {
		if done, ok := e.Event.(lingara.PracticeCompleted); ok && e.Known {
			return enqueue(ctx, e.PlayerRef, done) // keep it quick: enqueue and return
		}
		return nil
	}))
})
```

`Handle` verifies the signature first, so a forged event never reaches your
directory, then looks the subject up and calls your handler. It returns the
status to answer, always with an empty body:

| Outcome | Status |
|---|---|
| your handler returned `nil` | `204` |
| a bad signature (missing header, stale timestamp, no matching signature) | `401` |
| a valid signature over a malformed payload | `400` |
| your handler or your directory returned an error | `500` |

Lingara redelivers every non-`2xx`. The cause of a `500` goes to the router's
`onError`, by default `log/slog`'s default logger; pass your own with
`router.WithOnError(func(ctx context.Context, err error) { … })`.

- **Deduplicate on `e.Event.Meta().ID`.** Delivery is at least once.
- `e.Known` is false, and `e.PlayerRef` empty, for a subject your directory
  does not know: your own account's events, or a player you deleted.
- An event newer than the library arrives as `lingara.UnknownEvent`; answer
  it like any other.
- Delivery expects an answer within 10 s. A slow handler should enqueue and
  return.
- During a secret rotation pass both: `NewPlayerRouter(players, newSecret, oldSecret)`.
- `router.Route` gives you the routed event without the status mapping.

## Delete a player

```go
if err := embed.DeletePlayer(ctx, playerRef); err != nil {
	return err // the library's error; the directory row is kept
}
```

Call it from your own account-deletion path. It revokes every token the player
holds, then calls `Forget(playerRef)` on your directory. An unknown player is
a success, so it is safe to retry. Deleting your own data about the player is
yours.

## The player directory

`NewMemoryDirectory()` is lost on restart and not shared between instances:
use it for tests and single-process demos only. In production implement
`PlayerDirectory` over your own database, beside your player rows:

```sql
CREATE TABLE lingara_players (
  player_ref TEXT NOT NULL,
  subject    TEXT NOT NULL
);
CREATE UNIQUE INDEX lingara_players_subject ON lingara_players (subject);
CREATE INDEX lingara_players_player_ref ON lingara_players (player_ref);
```

```go
type PlayerDirectory interface {
	Remember(ctx context.Context, playerRef, subject string) error           // upsert keyed on subject
	PlayerRefFor(ctx context.Context, subject string) (string, bool, error) // false: not one of yours
	Forget(ctx context.Context, playerRef string) error                     // every row for the player
}
```

The `subject` is a pairwise identifier Lingara computes, so the mint's answer
is the only moment a player and their subject meet: that is why the kit
records it for you there. Unique on `subject` only, never on `player_ref`: a
player who later links a Lingara account owns a second subject beside the
first. A `snippets/server/go/webhook.go` sketch implements it over
`database/sql`.

## Versions

`lingaraembed.Version` is this kit's release, the embed SDK's one version. The
kit needs the `lingara` library at the floor in `go.mod` or newer.

## License

MIT.
