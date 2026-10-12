<?php

declare(strict_types=1);

namespace Lingara\Embed;

/**
 * A process-local PlayerDirectory, for tests and single-process demos. It
 * is lost when the process ends (under PHP-FPM, at the end of every
 * request) and is not shared between workers or instances: production code
 * implements PlayerDirectory over its own database.
 */
final class MemoryPlayerDirectory implements PlayerDirectory
{
    /** @var array<string, string> subject => player_ref */
    private array $rows = [];

    public function remember(string $playerRef, string $subject): void
    {
        $this->rows[$subject] = $playerRef;
    }

    public function playerRefFor(string $subject): ?string
    {
        return $this->rows[$subject] ?? null;
    }

    public function forget(string $playerRef): void
    {
        $this->rows = array_filter($this->rows, static fn(string $owner): bool => $owner !== $playerRef);
    }

    /**
     * Every row, subject => player_ref, sorted by subject.
     *
     * @return array<string, string>
     */
    public function snapshot(): array
    {
        $rows = $this->rows;
        ksort($rows, SORT_STRING);
        return $rows;
    }

    /** @return array<string, mixed> */
    public function __debugInfo(): array
    {
        return ['rows' => count($this->rows)];
    }
}
