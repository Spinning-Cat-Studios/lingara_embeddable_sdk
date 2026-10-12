<?php

declare(strict_types=1);

namespace Lingara\Embed\ConformanceEs;

use Lingara\Embed\MemoryPlayerDirectory;
use Lingara\Embed\PlayerDirectory;

/**
 * The case's MemoryPlayerDirectory with its lookups counted: a lookup
 * changes no contents, so only a count shows that a forged delivery never
 * reached the store.
 */
final class Recording implements PlayerDirectory
{
    public int $lookups = 0;
    public readonly MemoryPlayerDirectory $inner;

    /** @param array<string, string> $rows subject => player_ref */
    public function __construct(array $rows = [])
    {
        $this->inner = new MemoryPlayerDirectory();
        foreach ($rows as $subject => $playerRef) {
            $this->inner->remember($playerRef, (string) $subject);
        }
    }

    public function remember(string $playerRef, string $subject): void
    {
        $this->inner->remember($playerRef, $subject);
    }

    public function playerRefFor(string $subject): ?string
    {
        $this->lookups++;
        return $this->inner->playerRefFor($subject);
    }

    public function forget(string $playerRef): void
    {
        $this->inner->forget($playerRef);
    }

    /** @return array<string, string> */
    public function snapshot(): array
    {
        return $this->inner->snapshot();
    }
}
