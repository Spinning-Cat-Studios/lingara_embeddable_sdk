# `dotnet/`

The C# packages of the embed kits, built from one solution, `Lingara.Embed.sln`.

## Player: `Lingara.Embed`

For Unity and Godot C# games. It wraps the C ABI in [`../ffi/`](../ffi/README.md)
and ships the five native libraries inside the nupkg. See
[`src/Lingara.Embed/README.md`](src/Lingara.Embed/README.md) for install,
the poll loop and the token flow.

| Path | What it is |
|---|---|
| `src/Lingara.Embed/` | The package: `netstandard2.1` (Unity) and `net8.0` (Godot, NativeAOT). No `PackageReference`, no reflection |
| `tests/Lingara.Embed.Tests/` | xUnit against the real host library (`make test-dotnet`) |
| `tests/Lingara.Embed.AotSmoke/` | NativeAOT publish with zero warnings, as the stand-in for IL2CPP (`make dotnet-aot-smoke`) |
| `tests/Lingara.Embed.Bench/` | `Drain` of 256 events, timed (`make dotnet-bench`) |
| `tests/Lingara.Embed.GodotSnippet/` | Builds `snippets/player/godot-csharp/` against `Godot.NET.Sdk` (`make dotnet-godot-snippet`) |
| `conformance-ek/` | The EK harness (`make conformance-ek-dotnet`) |
| `samples/unity/`, `samples/godot/` | Minimal samples. Each carries a byte-identical copy of its snippet (`make check-dotnet-samples`) |

`make dotnet-pack` writes `artifacts/Lingara.Embed.<version>.nupkg`. It needs
all five `dist/ffi/<rid>/` directories (`make ffi-dist RID=<rid>`) and refuses
to pack, naming the platform, if any one is missing.

### Updates

An update notice is for you, the developer, and is never drawn for the
player. Set `EmbedOptions.EmbedVersion = "1.1.0"` to the embed version this
build was registered against: the Http source sends it to your endpoint as
`Lingara-Embed-Version`, and a polling game sends that header itself and
passes the handoff's notice as `ProvideToken(token, expiresAt, new
UpdateAvailable { … })`. Once per handle `Drain` yields an
`EmbedEvent.UpdateAvailable` (`Latest`, `Deprecated`, `Message`), beside a
`Log` at `Warn`; both samples log it.

## Server: `Lingara.Embed.Server`

For your game's or site's own server, the one that holds your Lingara client
credentials. Vouching for a player is four small jobs, and this package does
each in a call: **mint** a player's token and hand it over, render the
**widget** snippet, **route** a verified webhook to the player it concerns,
and **delete** a player (ADR 1.10.26af).

C# has no `lingara` API library, so this package carries the smallest HTTP
transport the two calls need (the client-credentials token cache, the
`Retry-After` retries, the error family) and its own Standard Webhooks
verifier, held to the libraries' shared vectors and to the same ES
conformance cases as the other seven server kits. It has **no package
dependency** at all, targets `net8.0` only, and shares nothing with the
player half above.

### Install

```xml
<PackageReference Include="Lingara.Embed.Server" Version="…" />
```

```csharp
using Lingara.Embed.Server;

var lingara = new EmbedServer(new EmbedServerOptions
{
    ClientId = Environment.GetEnvironmentVariable("LINGARA_CLIENT_ID")!,         // lgr_cid_…
    ClientSecret = Environment.GetEnvironmentVariable("LINGARA_CLIENT_SECRET")!, // lgr_cs_…
}, new SqlPlayerDirectory(db)); // see "The player directory"
```

Make one `EmbedServer` for the process: it caches the client-credentials token
and refreshes it before it expires. Pass your own `HttpClient` as the third
argument if you manage one (from `IHttpClientFactory`, say). Only a
**metered** client may mint (`403 embed_needs_metered` otherwise).

### Mint, and hand the token over

Your own route, behind your own session. The widget calls it with the page's
cookies; a game calls it however it talks to your server.

```csharp
app.MapPost("/lingara/token", async (ClaimsPrincipal user, CancellationToken ct) =>
{
    var playerRef = user.FindFirstValue(ClaimTypes.NameIdentifier); // your user, never a request value
    if (playerRef is null) return Results.Unauthorized();
    var minted = await lingara.MintTokenAsync(playerRef, new MintOptions { Origin = "https://game.example" }, ct);
    return Results.Text(TokenHandoff.Body(minted), "application/json"); // {"token":"lgr_et_…","expires_at":"…"}
}).RequireAuthorization().RequireRateLimiting("lingara-token");
```

- `MintTokenAsync` is one `POST /v1/embed/tokens` every time: the kit keeps
  **no cache**, so a token you revoked on one instance is never handed out by
  another. **Rate-limit this route per session yourself**; Lingara's own
  per-player limits cap your client, not your users.
- `Origin` is your page's origin, one of your client's registered embed
  origins. The kit never reads it from a request. A game's server omits it.
  `Scopes` narrows the token; none asks for every handable scope.
- On success the kit calls `RememberAsync(playerRef, minted.Subject)` on your
  directory. On failure it remembers nothing and throws: a
  `LingaraApiException` (`Status`, `Code`, `RetryAfter`), an `OAuthException`
  from the token endpoint, or a `MaintenanceException`, all
  `LingaraException`s. A `429` or `503` with a `Retry-After` of at most 60 s
  has already been retried, three attempts in all.
- `TokenHandoff.Body` writes exactly `{"token", "expires_at"}`, the body every
  player kit and the widget read. It is the one place the token is exposed:
  a `MintedToken` has no token property, and its `ToString()` and debugger
  display render `[REDACTED]`. `ExposeToken()` is the one accessor.

### The widget snippet

```csharp
var html = WidgetSnippet.Render(new SnippetOptions
{
    PlanId = planId,                 // the plan your server made for this player
    TokenEndpoint = "/lingara/token", // the route above
    Theme = "auto",
    Nonce = cspNonce,                // your request's CSP nonce
});
```

Three lines: the `<lingara-embed>` element, its module script (pinned to this
package's release), and an inline module that points the element's
`tokenProvider` at your endpoint. The token itself never appears in the
markup. A refused option is a `SnippetException` whose `Field` names it
(`planId`, `nonce`, …).

- **CSP.** If your `script-src` allows no inline script, pass the request's
  nonce: without it a strict page silently blocks the third line.
- **Same-origin credentials.** The element fetches your endpoint with
  `credentials: "same-origin"`, so your session cookie goes with it. An
  absolute `https://` endpoint on another origin receives no cookies and must
  authenticate the player some other way; the kit never widens this to
  `include`.

### Route a webhook to its player

```csharp
var router = new PlayerRouter(players, Environment.GetEnvironmentVariable("LINGARA_WEBHOOK_SECRET")!);
// a malformed lgr_whsec_… throws here, at startup; pass two secrets during a rotation

app.MapPost("/lingara/webhook", async (HttpRequest request, CancellationToken ct) =>
{
    using var body = new MemoryStream();
    await request.Body.CopyToAsync(body, ct); // the raw bytes: verify before anything parses them
    var status = await router.HandleAsync(body.ToArray(), name => request.Headers[name].FirstOrDefault(),
        (routed, token) => Enqueue(routed.PlayerRef, routed.Event, token), ct); // keep it quick
    return Results.StatusCode(status);
});
```

`HandleAsync` verifies the signature first, so a forged event never reaches
your directory, then looks the subject up and calls your handler. It returns
the status to answer, always with an empty body:

| Outcome | Status |
|---|---|
| your handler returned | `204` |
| a bad signature, header or timestamp | `401`; neither your directory nor your handler is called |
| a valid signature over a malformed payload | `400` |
| your handler or your directory threw | `500`; the exception goes to `OnError` |

- `OnError` defaults to `System.Diagnostics.Trace.TraceError`; set
  `new PlayerRouter(…) { OnError = e => logger.LogError(e, "lingara webhook") }`
  to use your own logger.
- Lingara redelivers every non-`2xx`, and delivery is **at least once**:
  **deduplicate on `routed.Event.Id`**. Delivery expects a `2xx` within 10 s,
  so enqueue slow work and return.
- `routed.PlayerRef` is null for a subject your directory does not know: your
  own client's events, or a player you deleted.
- `routed.Event` is a `WebhookEvent` (`Id`, `Type`, `CreatedAt`, `ApiVersion`,
  `Subject`, and `Data` as a `JsonElement`). A type newer than this package
  is `IsUnknown` and still routed: answer it `2xx` like any other.
- `RouteAsync` does the same without the status mapping, for routes that map
  failures themselves; `Webhooks.WebhookVerifier` is the verifier alone.

### Delete a player

```csharp
await lingara.DeletePlayerAsync(playerRef, ct);
```

Call it from your own account-deletion path. It revokes every token the
player holds, then calls `ForgetAsync` on your directory. An unknown player is
a success, so it is safe to repeat; on a failure the row is kept, because the
player still exists at Lingara. Deleting your own player data is yours.

### The player directory

`MemoryPlayerDirectory` is lost on restart and not shared between instances:
use it for tests and single-process demos only. In production, implement
`IPlayerDirectory` over your own database, beside your player rows:

```sql
CREATE TABLE lingara_players (
  player_ref text NOT NULL,
  subject    text PRIMARY KEY   -- unique on subject only
);
CREATE INDEX ON lingara_players (player_ref);
```

| Column | What it holds |
|---|---|
| `player_ref` | your player's id, as you pass it to `MintTokenAsync` |
| `subject` | the pairwise `lgr_sub_…` the mint returned, and events carry |

`RememberAsync` is an upsert keyed on `subject`; `PlayerRefForAsync` returns
the player or null; `ForgetAsync` removes every row for a player. A player who
links a Lingara account gains a second subject beside the first, which is why
the table is unique on `subject` and never on `player_ref`. A sketch over
`DbDataSource` is in [`snippets/server/csharp/`](../snippets/server/csharp/).

| Path | What it is |
|---|---|
| `src/Lingara.Embed.Server/` | The package: `net8.0`, no `PackageReference`, AOT-compatible, analysers with warnings as errors |
| `tests/Lingara.Embed.Server.Tests/` | xUnit: the snippet and handoff vectors, the router, E7's webhook vectors, the token cache, the package shape (`make test-dotnet-server`) |
| `tests/Lingara.Embed.Server.Snippets/` | Builds `snippets/server/csharp/` against the package (`make test-dotnet-server`) |
| `conformance-es/` | The ES harness on ASP.NET Core minimal APIs (`make conformance-es-dotnet`) |

Neither lane needs the native host library.
