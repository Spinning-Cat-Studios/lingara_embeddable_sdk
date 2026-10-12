<?php

declare(strict_types=1);

namespace Lingara\Embed\ConformanceEs;

/**
 * What the case driver (harness.php) and the webhook receiver
 * (receiver.php, under `php -S`) share: the armed case, the directory's
 * rows, and what each delivery's handler saw. One JSON file, read and
 * written under an exclusive flock; and the log sink, one file that both
 * processes' error_log() and the routers' onError append to.
 *
 * @phpstan-type Seen array{player_ref: string|null, event_id: string, unknown: bool}
 * @phpstan-type Armed array{
 *     deliver: list<mixed>,
 *     directory: array<string, string>,
 *     seen: array<int, Seen|null>,
 *     received: int,
 *     answered: int,
 *     lookups: int,
 * }
 */
final class SharedState
{
    private function __construct(
        public readonly string $dir,
    ) {}

    public static function create(): self
    {
        $dir = sys_get_temp_dir() . '/lingara-embed-es-php-' . getmypid() . '-' . bin2hex(random_bytes(4));
        if (!mkdir($dir, 0700) && !is_dir($dir)) {
            throw new \RuntimeException("cannot create {$dir}");
        }
        touch($dir . '/sink.log');
        return new self($dir);
    }

    public static function open(string $dir): self
    {
        return new self($dir);
    }

    public function sinkPath(): string
    {
        return $this->dir . '/sink.log';
    }

    /** Appends one line to the sink. */
    public function sink(string $line): void
    {
        file_put_contents($this->sinkPath(), $line . "\n", FILE_APPEND | LOCK_EX);
    }

    /**
     * Empties the sink and returns what it held.
     *
     * @return list<string>
     */
    public function takeSink(): array
    {
        $lines = file($this->sinkPath(), FILE_IGNORE_NEW_LINES | FILE_SKIP_EMPTY_LINES);
        file_put_contents($this->sinkPath(), '');
        return $lines === false ? [] : $lines;
    }

    /**
     * Runs `$change` over the armed case (null when nothing is armed) under
     * the lock, writes back what it returns (null disarms), and returns the
     * value `$change` put in `$out`.
     *
     * @param callable(Armed|null, mixed&): (Armed|null) $change
     */
    public function update(callable $change): mixed
    {
        return $this->locked(function () use ($change): mixed {
            $out = null;
            $next = $change($this->read(), $out);
            $next === null ? @unlink($this->path()) : file_put_contents($this->path(), json_encode($next, JSON_THROW_ON_ERROR));
            return $out;
        });
    }

    /** @return Armed|null */
    public function armed(): ?array
    {
        return $this->locked($this->read(...));
    }

    public function remove(): void
    {
        foreach (glob($this->dir . '/*') ?: [] as $file) {
            @unlink($file);
        }
        @rmdir($this->dir);
    }

    /**
     * @template T
     *
     * @param callable(): T $body
     *
     * @return T
     */
    private function locked(callable $body): mixed
    {
        $lock = fopen($this->dir . '/lock', 'c');
        if ($lock === false || !flock($lock, LOCK_EX)) {
            throw new \RuntimeException('cannot lock the shared state');
        }
        try {
            return $body();
        } finally {
            flock($lock, LOCK_UN);
            fclose($lock);
        }
    }

    private function path(): string
    {
        return $this->dir . '/armed.json';
    }

    /** @return Armed|null */
    private function read(): ?array
    {
        $text = @file_get_contents($this->path());
        if ($text === false || $text === '') {
            return null;
        }
        /** @var Armed $armed */
        $armed = json_decode($text, true, 512, JSON_THROW_ON_ERROR);
        return $armed;
    }
}
