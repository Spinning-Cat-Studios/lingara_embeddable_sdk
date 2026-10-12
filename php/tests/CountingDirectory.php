<?php

declare(strict_types=1);

namespace Lingara\Embed\Tests;

use Lingara\Embed\MemoryPlayerDirectory;
use Lingara\Embed\PlayerDirectory;

/** A MemoryPlayerDirectory whose lookups are counted, and which can be made to throw. */
final class CountingDirectory implements PlayerDirectory
{
    public int $lookups = 0;
    public bool $throws = false;
    private MemoryPlayerDirectory $inner;

    public function __construct()
    {
        $this->inner = new MemoryPlayerDirectory();
    }

    public function remember(string $playerRef, string $subject): void
    {
        $this->inner->remember($playerRef, $subject);
    }

    public function playerRefFor(string $subject): ?string
    {
        $this->lookups++;
        if ($this->throws) {
            throw new \RuntimeException('the directory is down');
        }
        return $this->inner->playerRefFor($subject);
    }

    public function forget(string $playerRef): void
    {
        $this->inner->forget($playerRef);
    }
}
