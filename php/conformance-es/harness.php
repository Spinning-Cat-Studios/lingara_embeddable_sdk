<?php

declare(strict_types=1);

// The case driver of the PHP server kit's ES harness (ADR 1.10.26af D13;
// conformance/es/CONTRACT.md, *What a harness reports*), started by
// serve.php once its webhook receiver is listening. For each case it
// fetches the case, builds a fresh library client, EmbedServer and
// MemoryPlayerDirectory, arms it, runs the `kit:` jobs, waits for every
// delivery, judges what only the caller sees, finishes, and writes one
// result line. There is no skip. It drives the kit's public API only.

namespace Lingara\Embed\ConformanceEs;

require_once __DIR__ . '/lib/SharedState.php';
require_once __DIR__ . '/lib/Recording.php';
require_once __DIR__ . '/lib/Control.php';
require_once __DIR__ . '/lib/Jobs.php';
require_once __DIR__ . '/lib/Judge.php';
require_once __DIR__ . '/lib/Receiver.php';

use Lingara\Client;
use Lingara\Embed\EmbedServer;
use Lingara\HttpStack;

final class Harness
{
    /** The language this harness reports: make/php.mk's ES_LANGS entry. */
    public const LANG = 'php';

    private readonly Control $control;

    public function __construct(
        private readonly string $variant,
        private readonly SharedState $state,
    ) {
        $this->control = new Control(self::env('LINGARA_EMBED_SERVER_URL'));
    }

    /** Every case, one result line each; the exit status. */
    public function runAll(): int
    {
        // The library's default logger, and the router's default onError,
        // are error_log(): into the sink, as the receiver's are.
        ini_set('log_errors', '1');
        ini_set('error_log', $this->state->sinkPath());
        $out = fopen(self::env('LINGARA_EMBED_RESULTS'), 'w');
        if ($out === false) {
            return 1;
        }
        foreach (self::caseIds((string) (getenv('LINGARA_EMBED_CASES') ?: 'conformance/es/cases')) as $id) {
            fwrite($out, json_encode($this->runCase($id), JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES) . "\n");
        }
        fclose($out);
        return 0;
    }

    public static function env(string $name): string
    {
        $value = getenv($name);
        if (!is_string($value) || $value === '') {
            throw new \RuntimeException("{$name} is not set");
        }
        return $value;
    }

    /**
     * `<group>/<slug>.yaml` → `<group>.<slug>`, sorted.
     *
     * @return list<string>
     */
    private static function caseIds(string $dir): array
    {
        $ids = [];
        foreach (glob($dir . '/*/*.yaml') ?: [] as $file) {
            $ids[] = basename(dirname($file)) . '.' . basename($file, '.yaml');
        }
        sort($ids, SORT_STRING);
        return $ids;
    }

    /** @return array{case: string, lang: string, result: string, mismatches: list<string>} */
    private function runCase(string $id): array
    {
        try {
            $case = $this->control->fetchCase($id);
        } catch (\RuntimeException $e) {
            return self::result($id, ["cannot read the case: {$e->getMessage()}"]);
        }
        $directory = new Recording(Judge::stringMap(self::member(self::member($case, 'directory'), 'seed')));
        $deliver = array_values(self::member($case, 'deliver'));
        Receiver::arm($this->state, $deliver, $directory->snapshot());
        $this->state->takeSink();
        $server = new EmbedServer($this->client(), $directory);
        $mismatches = $this->control->post('arm', $id);
        $outcome = Jobs::run($server, self::member($case, 'kit'), $directory, $this->state);
        $armed = $this->waitForDeliveries(count($deliver));
        $judge = new Judge($id, $directory, $armed);
        array_push($mismatches, ...$judge->expectations(self::member($case, 'expect'), $outcome));
        array_push($mismatches, ...$judge->handlers($deliver));
        if ($outcome->render) {
            array_push($mismatches, ...$judge->renderings($this->state, [$server, Receiver::router($this->state, ['router' => ['secrets' => ['primary', 'secondary']]])], $outcome));
        }
        array_push($mismatches, ...$this->control->finish($id, $judge->handlerPlayers(count($deliver))));
        $this->state->update(static fn(): ?array => null);
        return self::result($id, $mismatches);
    }

    /**
     * `$value[$key]` when both are arrays, else empty.
     *
     * @param array<mixed> $value
     *
     * @return array<mixed>
     */
    private static function member(array $value, string $key): array
    {
        return is_array($value[$key] ?? null) ? $value[$key] : [];
    }

    /**
     * @param list<string> $mismatches
     *
     * @return array{case: string, lang: string, result: string, mismatches: list<string>}
     */
    private static function result(string $id, array $mismatches): array
    {
        return ['case' => $id, 'lang' => self::LANG, 'result' => $mismatches === [] ? 'pass' : 'fail', 'mismatches' => $mismatches];
    }

    /** A fresh client per case, so no token outlives its case. */
    private function client(): Client
    {
        $base = self::env('LINGARA_EMBED_SERVER_URL');
        return new Client(
            clientId: self::env('LINGARA_EMBED_CLIENT_ID'),
            clientSecret: self::env('LINGARA_EMBED_CLIENT_SECRET'),
            baseUrl: $base,
            tokenUrl: $base . '/oauth/token',
            http: $this->variant === 'guzzle' ? HttpStack::guzzle() : HttpStack::symfony(),
        );
    }

    /**
     * Until every delivery has been answered, or 15 s; then the armed state.
     *
     * @return array<string, mixed>
     */
    private function waitForDeliveries(int $count): array
    {
        $deadline = microtime(true) + 15;
        while (true) {
            $armed = $this->state->armed() ?? [];
            if (($armed['answered'] ?? 0) >= $count || microtime(true) > $deadline) {
                return $armed;
            }
            usleep(20_000);
        }
    }
}
