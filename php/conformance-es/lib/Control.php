<?php

declare(strict_types=1);

namespace Lingara\Embed\ConformanceEs;

/**
 * The conformance server's control routes: `GET /__conformance/cases/{id}`,
 * `POST /__conformance/arm/{id}` and `POST /__conformance/finish/{id}`.
 * Plain stream wrappers: the harness adds no HTTP client of its own.
 */
final class Control
{
    public function __construct(
        private readonly string $server,
    ) {}

    /**
     * @return array<string, mixed>
     *
     * @throws \RuntimeException
     */
    public function fetchCase(string $id): array
    {
        [$status, $body] = $this->send('GET', 'cases', $id, null);
        $case = $status === 200 ? json_decode($body, true) : null;
        if (!is_array($case)) {
            throw new \RuntimeException("GET cases/{$id} answered {$status}");
        }
        /** @var array<string, mixed> $case */
        return $case;
    }

    /** @return list<string> a mismatch when the verb failed */
    public function post(string $verb, string $id): array
    {
        [$status] = $this->send('POST', $verb, $id, null);
        return $status >= 200 && $status < 300 ? [] : ["cannot {$verb}: status {$status}"];
    }

    /**
     * @param list<string|null> $handlers
     *
     * @return list<string> the server's mismatches
     */
    public function finish(string $id, array $handlers): array
    {
        [$status, $body] = $this->send('POST', 'finish', $id, json_encode(['handlers' => $handlers], JSON_THROW_ON_ERROR));
        $answer = json_decode($body, true);
        if ($status < 200 || $status >= 300 || !is_array($answer)) {
            return ["cannot finish: status {$status}"];
        }
        $found = [];
        foreach (is_array($answer['mismatches'] ?? null) ? $answer['mismatches'] : [] as $mismatch) {
            $found[] = is_string($mismatch) ? $mismatch : (string) json_encode($mismatch);
        }
        return $found;
    }

    /** @return array{int, string} */
    private function send(string $method, string $verb, string $id, ?string $json): array
    {
        $http = ['method' => $method, 'ignore_errors' => true, 'timeout' => 30, 'header' => "content-type: application/json\r\n"];
        if ($json !== null) {
            $http['content'] = $json;
        }
        $stream = @fopen("{$this->server}/__conformance/{$verb}/{$id}", 'r', false, stream_context_create(['http' => $http]));
        if ($stream === false) {
            return [0, ''];
        }
        // wrapper_data holds the response's header lines on every PHP from 8.2.
        $headers = stream_get_meta_data($stream)['wrapper_data'];
        $body = stream_get_contents($stream);
        fclose($stream);
        $status = 0;
        foreach (is_array($headers) ? $headers : [] as $line) {
            if (is_string($line) && preg_match('#^HTTP/\S+ (\d{3})#', $line, $m) === 1) {
                $status = (int) $m[1];
            }
        }
        return [$status, $body === false ? '' : $body];
    }
}
