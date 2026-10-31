# @lingara/embed

Lingara practice inside your website: one tag, one function. The tag frames
Lingara's hosted practice page; the function asks **your own server** for a
short-lived token. Everything between the two (the handshake, re-minting,
resizing and events) is the element's job.

- No runtime dependencies, ESM only, under 5 KiB gzip.
- Importing it on a server does nothing: no element is defined and no DOM
  global is touched until you call `defineLingaraEmbed()` in a browser.
- The token never appears in a URL, an attribute, an event or a log.

## Install

```sh
npm install @lingara/embed
```

No bundler? Load the self-registering script build, pinned to a version:

```html
<script type="module"
        src="https://cdn.jsdelivr.net/npm/@lingara/embed@<version>/dist/lingara-embed.min.js"
        integrity="sha384-…" crossorigin="anonymous"></script>
```

The `integrity` value ships in the package beside the file, as
`dist/lingara-embed.min.js.sri`: read it from your installed copy or from
`https://cdn.jsdelivr.net/npm/@lingara/embed@<version>/dist/lingara-embed.min.js.sri`.

## Mount

```html
<lingara-embed surface="practice" plan-id="7d0c…" theme="auto" locale="fr" title="Pratique"></lingara-embed>
<script type="module">
  import { defineLingaraEmbed } from '@lingara/embed'
  defineLingaraEmbed()
  const widget = document.querySelector('lingara-embed')
  widget.tokenProvider = async () =>
    (await fetch('/lingara/token', { method: 'POST', credentials: 'same-origin' })).json()
  widget.addEventListener('lingara:practice-completed', (e) => console.log(e.detail.correct, e.detail.total))
</script>
```

`import '@lingara/embed/define'` is the same as importing and calling
`defineLingaraEmbed()`. Calling it twice is harmless.

| Attribute | Values | Default | Changing it after mount |
|---|---|---|---|
| `surface` | `practice` | `practice` | reloads the frame |
| `plan-id` | a lesson plan id (a UUID) your server made for this player | required | reloads the frame |
| `theme` | `light` · `dark` · `auto` | `auto` | applied without a reload |
| `locale` | a BCP 47 tag | the visitor's browser languages | applied without a reload |
| `lingara-origin` | `https://…`, or `http://localhost[:port]` / `http://127.0.0.1[:port]` | `https://app.getlingara.com` | reloads the frame |
| `title` | the frame's accessible name | `Lingara practice` (English) | updates the frame |
| `loading` | `lazy` · `eager` | `lazy` | read at the next frame build |

**Set `title` in your page's language.** The element cannot know it, so its
default is English.

Each attribute except `title` is also a string property (`surface`, `planId`,
`theme`, `locale`, `lingaraOrigin`, `loading`). The element is a block 480 px
tall until the page inside reports its height; your own CSS `height` or
`max-height` on `lingara-embed` always wins.

**Moving the element in the DOM reloads it**, and may mint a new token: a move
is a disconnect and a reconnect.

## `tokenProvider`

```ts
type TokenGrant = { token: string; expires_at: string }
type TokenProvider = () => Promise<TokenGrant>
```

Your endpoint mints an embed token on your server, **with this page's
origin**, and returns exactly `{"token", "expires_at"}`, the body the
`/server` half of this package (and every Lingara server kit) writes for you.
The element reads only those two fields. It calls the provider when the frame
first asks, re-mints about a minute before expiry, and backs off 2 s, 8 s and
30 s between failed attempts before reporting `token_provider_failed`.

## Events

All are `CustomEvent`s that bubble and cross shadow roots.

| Event | `detail` |
|---|---|
| `lingara:ready` | `{ surface, planId }` |
| `lingara:resize` | `{ height }` (120–4000) |
| `lingara:practice-completed` | `{ planId, setNumber, correct, total, eventId }` |
| `lingara:error` | `{ code, retryable }` |
| `lingara:update-available` | `{ latest, deprecated, message }`: once per element session (connect to disconnect), for you, never for the visitor |

**An update notice never reaches the visitor or the frame.** Your
`tokenProvider` is your code, so it sends `Lingara-Embed-Version: <the embed
version this page was registered against>` to your token route itself; when
your server forwards the mint's `update_available` in the grant, the element
dispatches `lingara:update-available` once and writes one `console.warn` line.
A malformed `update_available` is dropped and the grant still works.

**`eventId` is not the webhook's id.** It is the id of the record that stored
the result (`null` if storing it failed). The `practice.completed` webhook your
server hears is a separate delivery with its own id; to join the two, key on
the player (the webhook's `subject`), `data.plan_id` and each item's
`set_number`.

### Error codes

Every `lingara:error` ends the session; the element never remounts on its own.
When `retryable` is `true`, calling `widget.reload()` may succeed; when to do so
is yours to decide.

| `code` | `retryable` | Meaning |
|---|---|---|
| `origin_refused` | from the page | the token was bound to a different registered origin than this page's |
| `token_invalid` · `token_expired` · `scope_missing` | from the page | the token was refused |
| `plan_not_found` | `false` | `plan-id` is missing or not a UUID, or the plan is not this player's |
| `plan_not_ready` · `embed_unavailable` · `rate_limited` · `network` · `internal` | from the page | the practice page could not continue |
| `surface_unsupported` | `false` | `surface` is not one the page serves |
| `version_unsupported` | `false` | the page speaks no protocol version this element does |
| `lingara_origin_invalid` | `false` | `lingara-origin` is not a valid origin |
| `token_provider_invalid` | `true` | your provider returned something other than `{token, expires_at}` |
| `token_provider_failed` | `true` | your provider failed four times in a row |

### Troubleshooting

| You see | Usually because |
|---|---|
| `token_provider_failed` | your endpoint failed. An origin missing from your client's embed origins is refused **at the mint** (`400 embed_origin_not_registered`), so it surfaces here, not as `origin_refused` |
| `origin_refused` | a token minted for one registered origin is being used on another page origin |
| `plan_not_found` at once, with no frame | `plan-id` is absent or not a UUID |

## React

React 19 sets `tokenProvider` as a property. React 18 passes unknown props as
attributes, so use a ref:

```tsx
function Practice({ planId }: { planId: string }) {
  const ref = useRef<LingaraEmbedElement>(null)
  useEffect(() => {
    defineLingaraEmbed()
    if (ref.current) ref.current.tokenProvider = fetchToken
  }, [])
  return <lingara-embed ref={ref} plan-id={planId} />
}
```

A server-rendered `<lingara-embed>` is inert markup until the client calls
`defineLingaraEmbed()`. A `tokenProvider` assigned before that is kept.

## Content Security Policy

On the page that mounts the element:

- `frame-src https://app.getlingara.com` (or your `lingara-origin`);
- `script-src https://cdn.jsdelivr.net` if you load the script build from it.

The element makes no network request of its own; your `tokenProvider` calls
your own origin.

## Server

`@lingara/embed/server` is the other half: the four jobs your server does so
a player can practise. It mints the player's token, hands it to the page,
routes Lingara's webhooks to the player they concern, and deletes a player.
It runs on Node ≥ 22 over the official API library, which it takes as an
optional peer, so a page that only mounts the element installs nothing more:

```sh
npm i @lingara/embed @lingara/api
```

Importing `@lingara/embed/server` without `@lingara/api` installed fails at
import, naming the missing package. ESM and CommonJS both work.

### The player directory

**`MemoryPlayerDirectory` is lost on restart and not shared between
instances**: use it in tests and single-process demos only. A player's
`subject` (the `lgr_sub_…` every event carries) is a pairwise value Lingara
computes, so the mint's answer is the only moment you see it beside your own
`playerRef`. Keep the pair in your own database, beside your players, and
implement `PlayerDirectory` over it (each method may return a value or a
promise):

| Column | Index |
|---|---|
| `player_ref` | a plain index: one player may own several subjects once they link an account |
| `subject` | **unique**: `remember` upserts on it |

```ts
class PlayerTable implements PlayerDirectory {
  async remember(playerRef: string, subject: string) {
    await db.query('INSERT INTO lingara_players (player_ref, subject) VALUES ($1, $2) ON CONFLICT (subject) DO UPDATE SET player_ref = $1', [playerRef, subject])
  }
  async playerRefFor(subject: string) {
    return (await db.query('SELECT player_ref FROM lingara_players WHERE subject = $1', [subject]))[0]?.player_ref
  }
  async forget(playerRef: string) {
    await db.query('DELETE FROM lingara_players WHERE player_ref = $1', [playerRef])
  }
}
```

### Mint a token

Your own route, behind your own sign-in. It mints with **this page's
origin** (one of your client's embed origins) and returns exactly
`{"token","expires_at"}`, the body the element reads. Every call is one
request to Lingara; there is no cache, so **rate-limit this route per
session**.

```ts
import express from 'express'
import { Lingara } from '@lingara/api'
import { EmbedServer, tokenHandoff } from '@lingara/embed/server'

const lingara = new Lingara({ clientId: process.env.LINGARA_CLIENT_ID, clientSecret: process.env.LINGARA_CLIENT_SECRET })
const server = new EmbedServer(lingara, { directory: new PlayerTable() })

app.post('/lingara/token', requireSignIn, async (req, res) => {
  const minted = await server.mintToken(req.user.id, { origin: 'https://www.example.com' })
  res.set('cache-control', 'no-store').type('application/json').send(tokenHandoff(minted))
})
```

A failed call raises the library's own error (`ApiError` with its `status`
and `code`, `OAuthError`, `MaintenanceError`), unchanged; nothing is
remembered. A game's server mints the same way, without `origin`.

### Render the widget

`widgetSnippet` writes the element and its two module scripts, escaped, for
your page. The token never appears in it: the element asks `tokenEndpoint`
at run time with `credentials: "same-origin"`, so your route sees the
visitor's cookies. An absolute endpoint on another origin receives none, and
must authenticate some other way.

```ts
import { widgetSnippet } from '@lingara/embed/server'

const html = widgetSnippet({ planId, tokenEndpoint: '/lingara/token', theme: 'auto', locale: 'fr', title: 'Pratique', nonce: res.locals.cspNonce })
```

**Pass your CSP nonce** if your `script-src` allows no inline script: the
third line is an inline module, and a strict page blocks it silently. An
invalid option throws a `SnippetError` whose `field` names it (`planId`,
`tokenEndpoint`, …).

### Route a webhook

`PlayerRouter` verifies the signature on the **raw** body before anything
else, then resolves the event's `subject` to your player, so a forged
delivery never reaches your store. `handle` returns the status to send, with
an empty body:

```ts
import { PlayerRouter } from '@lingara/embed/server'

const router = new PlayerRouter({ secrets: process.env.LINGARA_WEBHOOK_SECRET!, directory: new PlayerTable() })

app.post('/lingara/webhook', express.raw({ type: '*/*' }), async (req, res) => {
  const status = await router.handle(req.body, req.headers, async ({ event, playerRef }) => {
    if (await seenBefore(event.id)) return // delivery is at least once
    if (playerRef !== null) await jobs.enqueue(playerRef, event)
  })
  res.sendStatus(status)
})
```

| `handle` returns | When |
|---|---|
| `204` | your `onEvent` returned |
| `401` | the signature, timestamp or headers failed verification |
| `400` | the signature was valid but the payload was malformed |
| `500` | your directory or your `onEvent` threw; the error goes to `onError` (default `console.error`), never the body or a secret |

Lingara redelivers every non-`2xx`. **Deduplicate on `event.id`**, and answer
within 10 s: enqueue anything slow and return. `playerRef` is `null` for a
subject your directory does not know (your own, or a deleted player); an
event type newer than the library arrives as `UnknownEvent` and still needs
its `204`. During a secret rotation, pass both: `secrets: [newest, previous]`.
`route(body, headers)` is the same without the status mapping.

### Delete a player

From your own account-deletion path. It revokes every token the player
holds and then forgets their rows; an unknown player is a success, and on a
failure the rows are kept. Deleting your own data about them stays yours.

```ts
await server.deletePlayer(user.id)
```

## Licence

MIT.
