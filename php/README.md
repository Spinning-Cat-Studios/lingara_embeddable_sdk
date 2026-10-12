# spinningcatstudios/lingara-embed

The official Lingara embed server kit for PHP. Your server is the one that
vouches for each player: it mints the player's short-lived token, routes
Lingara's signed webhooks to the right player, deletes a player when their
account goes, and renders the widget snippet for your pages.

It depends on [`spinningcatstudios/lingara`](https://packagist.org/packages/spinningcatstudios/lingara)
and nothing else: the library's client makes the calls, its verifier checks
the webhooks, and its `Lingara\MintedToken` is what a mint returns. The
library sends through Symfony HttpClient or Guzzle 7, so install one. PHP 8.2
or newer.

## Install

```sh
composer require spinningcatstudios/lingara-embed symfony/http-client nyholm/psr7
```

While the kit and the library are pre-releases, a project on
`minimum-stability: stable` asks for them explicitly:
`composer require "spinningcatstudios/lingara-embed:@alpha" "spinningcatstudios/lingara:@alpha"`.

## The player directory

**`MemoryPlayerDirectory` is lost when the process ends and is not shared
between workers or instances** (under PHP-FPM that is every request), so it
is for tests and single-process demos only.

A webhook names its player by `subject`, a pairwise `lgr_sub_…` Lingara
computes. No kit can turn it back into your player id: the mint's answer is
the only moment both are in one place, so `EmbedServer` records the pair in a
`PlayerDirectory` you implement over your own database, beside your players:

| Column | |
|---|---|
| `player_ref` | your player id, with a plain index |
| `subject` | the `lgr_sub_…`, with a **unique** index |

```sql
CREATE TABLE lingara_players (player_ref TEXT NOT NULL, subject TEXT NOT NULL);
CREATE UNIQUE INDEX lingara_players_subject ON lingara_players (subject);
CREATE INDEX lingara_players_player_ref ON lingara_players (player_ref);
```

`remember()` upserts on `subject`; `playerRefFor()` returns the player or
`null`; `forget()` removes every row of the player. One player may own two
subjects once they link an account, so the store is unique on `subject` only.
[`snippets/server/php/webhook.php`](../snippets/server/php/webhook.php) has a
`PdoPlayerDirectory` to copy.

## Mint: your token route

```php
use Lingara\Client;
use Lingara\Embed\EmbedServer;
use Lingara\Embed\TokenHandoff;

$embed = new EmbedServer(
    new Client(clientId: getenv('LINGARA_CLIENT_ID') ?: null, clientSecret: getenv('LINGARA_CLIENT_SECRET') ?: null),
    new PdoPlayerDirectory($db),
);

// POST /lingara/token, behind your own login.
$playerId = $_SESSION['player_id'] ?? null;
if (!is_string($playerId)) { http_response_code(401); exit; }
$minted = $embed->mintToken($playerId, origin: 'https://game.example'); // this page's origin
header('Content-Type: application/json');
header('Cache-Control: no-store');
echo TokenHandoff::body($minted); // {"token":"lgr_et_…","expires_at":"…"}
```

- Every `mintToken()` is one `POST /v1/embed/tokens`; the kit keeps no cache.
  **Rate-limit this route per session.** Lingara caps each player too.
- `origin` is an argument, never read from the request: pass your page's
  origin, one of your client's registered embed origins. A game's server mints
  without one.
- A failed call throws the library's exception unchanged
  (`Lingara\Exception\ApiException` with `status()` and `errorCode()`, …),
  and nothing is remembered.
- The client credentials are your metered client's (`lgr_ci_…`, `lgr_cs_…`).
  Keep them in the environment, and give the client a PSR-16 `tokenCache:`
  under PHP-FPM.

`TokenHandoff::body()` is the one place the token is exposed, in exactly the
shape every player kit and the widget read. Return it and nothing else.

## Webhooks: route them to your players

```php
use Lingara\Embed\PlayerRouter;
use Lingara\Embed\RoutedEvent;
use Lingara\Events\Generated\PracticeCompleted;

$router = new PlayerRouter(new PdoPlayerDirectory($db), getenv('LINGARA_WEBHOOK_SECRET') ?: '');

// POST /lingara/webhook: the raw body, and getallheaders() or any PSR-7 request.
http_response_code($router->handle(file_get_contents('php://input'), getallheaders(), function (RoutedEvent $routed) {
    if ($routed->event instanceof PracticeCompleted && $routed->playerRef !== null) {
        recordPractice($routed->event->id, $routed->playerRef, $routed->event->data);
    }
}));
```

`handle()` verifies the **raw** body before anything else, so a forged event
never reaches your directory, and returns the status to send with an empty
body:

| Outcome | Status |
|---|---|
| your `onEvent` returned | `204` |
| a bad signature (any of the five reasons) | `401` |
| a valid signature over a malformed payload | `400` |
| your `onEvent` or your directory threw | `500` |

- Lingara redelivers every non-2xx. **Deduplicate on `$routed->event->id`**:
  delivery is at least once, and unordered.
- `playerRef` is `null` for a subject your directory does not know: your own
  (owner) subject, or a deleted player. An `UnknownEvent` (a type newer than
  the library) is routed like any other; acknowledge it.
- `onEvent` runs inline and Lingara expects the `2xx` within 10 s: enqueue
  slow work and return.
- A `500`'s cause goes to the router's third argument, `onError(\Throwable)`,
  by default `error_log()`. It never carries the body, a signature or a secret.
- During a secret rotation pass both: `new PlayerRouter($directory, [$new, $old])`.
  A malformed secret throws at construction.
- `route($body, $headers)` returns the `RoutedEvent` (or throws the library's
  `VerificationException`) for a route that maps statuses itself.

## Delete a player

```php
// From your account-deletion path.
$embed->deletePlayer($playerId);
```

It revokes every token of the player at Lingara (one
`DELETE /v1/embed/players/{player_ref}`), then forgets their rows. An unknown
player is a success too, so a retry is safe; on a failure the rows are kept.
Deleting your own player data stays yours.

## The widget snippet

```php
use Lingara\Embed\SnippetOptions;
use Lingara\Embed\WidgetSnippet;

echo WidgetSnippet::render(new SnippetOptions(
    planId: $plan->id,               // the plan your server made for this player
    tokenEndpoint: '/lingara/token', // your token route
    nonce: $cspNonce,                // your page's CSP nonce
));
```

It writes the `<lingara-embed>` element, the element's script at this kit's
release, and one inline module script that points the element at your token
route. The token never appears in the markup. Options: `planId` and
`tokenEndpoint` (required), `surface`, `theme`, `locale`, `lingaraOrigin`,
`title`, `id` (two widgets on one page need two), `nonce` and `elementSrc`. A
bad option throws `SnippetException`, whose `field` names it.

- **CSP nonce.** If your `script-src` allows no inline script, pass the
  request's nonce; without one a strict page blocks the third line silently.
- **Same-origin credentials.** The element asks your token route with
  `credentials: "same-origin"`, so it sends your session cookie only to your
  own origin. An absolute `tokenEndpoint` on another origin receives no
  cookies and must authenticate some other way.

## Development

```sh
make -C .. test-php            # PHPUnit, PHPStan, the PHPCS budgets, the snippets
make -C .. conformance-es-php  # every ES case, on nyholm/psr7 and on guzzlehttp/psr7
make -C .. codegen-php         # src/Version.php from ../VERSION
```

The ES harness (`conformance-es/`) runs its webhook receiver under `php -S`
through a port-0 launcher, once per PSR-7 stack.
