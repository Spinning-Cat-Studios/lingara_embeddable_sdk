<?php

declare(strict_types=1);

// The PHP server kit's examples the documentation site shows (ADR
// 1.10.26af D13). Each marked region is vendored at a released tag; `make
// test-php` parses every file here and type-checks it against the kit with
// PHPStan. A `use` line is legal only at namespace scope, so a key that
// shows one has two regions: the `use` lines, then the code.

namespace LingaraEmbedSnippets;

// lingara:begin mint
use Lingara\Client;
use Lingara\Embed\EmbedServer;
use Lingara\Embed\TokenHandoff;
// lingara:end
// lingara:begin mint
// Once, at startup. The credentials are your metered client's (lgr_ci_…,
// lgr_cs_…), from the environment; never ship them to a device.
function embedServer(\PDO $db): EmbedServer
{
    $client = new Client(
        clientId: getenv('LINGARA_CLIENT_ID') ?: null,
        clientSecret: getenv('LINGARA_CLIENT_SECRET') ?: null,
    );
    return new EmbedServer($client, new PdoPlayerDirectory($db));
}

// POST /lingara/token: your own route, behind your own login. Rate-limit it
// per session; every call is one mint.
function tokenRoute(EmbedServer $embed): void
{
    session_start();
    $playerId = $_SESSION['player_id'] ?? null;
    if (!is_string($playerId)) {
        http_response_code(401);
        return;
    }
    $minted = $embed->mintToken($playerId, origin: 'https://game.example');
    header('Content-Type: application/json');
    header('Cache-Control: no-store');
    echo TokenHandoff::body($minted); // {"token":"lgr_et_…","expires_at":"…"}
}
// lingara:end