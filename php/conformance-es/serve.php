<?php

declare(strict_types=1);

// The PHP server kit's ES harness entry (ADR 1.10.26af D13), run by
// `conformance-server run --tier es --lang php` with ROOT as its working
// directory:
//
//   php php/conformance-es/serve.php <nyholm|guzzle>
//
// PHP has a built-in server but no way to ask it for a free port and hear
// back which one, so this is the port-0 launcher: it finds a free loopback
// port, starts `php -S 127.0.0.1:<port>` over receiver.php (the webhook
// receiver), waits until it accepts a connection, prints `listening <port>`
// as its first line on standard output, and then drives every case through
// harness.php. The two processes share the armed case through a state
// directory (lib/SharedState.php).
//
// The variant picks the PSR-7 implementation the receiver builds each
// request with, and the HTTP stack the library sends through: `nyholm`
// (nyholm/psr7, Symfony HttpClient) or `guzzle` (guzzlehttp/psr7, Guzzle).
//
// The server runs under `sh`, which kills it when this process's stdin pipe
// to it closes: the conformance server may stop this launcher with SIGKILL,
// and an orphaned server would otherwise keep running.

require __DIR__ . '/../vendor/autoload.php';
require_once __DIR__ . '/harness.php';

use Lingara\Embed\ConformanceEs\Harness;
use Lingara\Embed\ConformanceEs\SharedState;

function fail(string $message): never
{
    // Standard error: standard output's first line is the handshake.
    fwrite(STDERR, "serve.php: {$message}\n");
    exit(1);
}

function freePort(): int
{
    $socket = stream_socket_server('tcp://127.0.0.1:0', $errno, $error) ?: fail($error ?? 'no free port');
    $name = (string) stream_socket_get_name($socket, false);
    fclose($socket);
    return (int) substr($name, (int) strrpos($name, ':') + 1);
}

/** @param resource $process */
function ready(int $port, mixed $process): bool
{
    $deadline = microtime(true) + 20;
    while (microtime(true) < $deadline && proc_get_status($process)['running']) {
        $probe = @fsockopen('127.0.0.1', $port, $errno, $error, 0.2);
        if ($probe !== false) {
            fclose($probe);
            return true;
        }
        usleep(20_000);
    }
    return false;
}

/** @return array{resource, array<int, resource>} */
function start(int $port, string $variant, SharedState $state): array
{
    $server = escapeshellarg(PHP_BINARY) . ' -q -d expose_php=0 -d log_errors=1 -d error_log=' . escapeshellarg($state->sinkPath())
        . " -S 127.0.0.1:{$port} " . escapeshellarg(__DIR__ . '/receiver.php');
    $command = ['sh', '-c', "{$server} <&- & s=\$!; cat > /dev/null; kill \$s"];
    $env = getenv();
    $env['LINGARA_EMBED_PSR7'] = $variant;
    $env['LINGARA_EMBED_ES_STATE'] = $state->dir;
    $process = proc_open($command, [0 => ['pipe', 'r'], 1 => STDERR, 2 => STDERR], $pipes, null, $env);
    $process !== false || fail('could not start php -S');
    return [$process, $pipes];
}

$variant = $argv[1] ?? 'nyholm';
in_array($variant, ['nyholm', 'guzzle'], true) || fail('the variant is nyholm or guzzle');
$state = SharedState::create();

// A port freed above can be taken before php -S binds it: try a few.
for ($attempt = 0; $attempt < 5; $attempt++) {
    $port = freePort();
    [$process, $pipes] = start($port, $variant, $state);
    if (ready($port, $process)) {
        fwrite(STDOUT, "listening {$port}\n");
        fflush(STDOUT);
        $status = (new Harness($variant, $state))->runAll();
        fclose($pipes[0]);
        proc_close($process);
        $state->remove();
        exit($status);
    }
    fclose($pipes[0]);
    proc_close($process);
}
$state->remove();
fail('php -S did not start');
