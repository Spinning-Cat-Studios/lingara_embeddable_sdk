<?php

declare(strict_types=1);

// The PHP server kit's webhook receiver (ADR 1.10.26af D13): the front
// controller `php -S` runs for every delivery, started by serve.php.
// LINGARA_EMBED_PSR7 chooses the PSR-7 implementation the request is built
// with, `nyholm` (nyholm/psr7 and nyholm/psr7-server) or `guzzle`
// (guzzlehttp/psr7); the kit's PlayerRouter reads its headers from that
// MessageInterface and its raw body from the stream.

require __DIR__ . '/../vendor/autoload.php';
require_once __DIR__ . '/harness.php';

use GuzzleHttp\Psr7\ServerRequest;
use Lingara\Embed\ConformanceEs\Harness;
use Lingara\Embed\ConformanceEs\Receiver;
use Lingara\Embed\ConformanceEs\SharedState;
use Nyholm\Psr7\Factory\Psr17Factory;
use Nyholm\Psr7Server\ServerRequestCreator;

// Every answer is an empty body; `php -S` would label it text/html.
ini_set('default_mimetype', '');

if (getenv('LINGARA_EMBED_PSR7') === 'guzzle') {
    $request = ServerRequest::fromGlobals();
} else {
    $factory = new Psr17Factory();
    $request = (new ServerRequestCreator($factory, $factory, $factory, $factory))->fromGlobals();
}
http_response_code(Receiver::deliver(SharedState::open(Harness::env('LINGARA_EMBED_ES_STATE')), $request));
