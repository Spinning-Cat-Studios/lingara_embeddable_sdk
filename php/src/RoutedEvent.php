<?php

declare(strict_types=1);

namespace Lingara\Embed;

use Lingara\Events\Event;

/**
 * A verified event and the player it concerns (ADR 1.10.26af D8).
 * `playerRef` is null for a subject the directory does not know: your own
 * (owner) subject, or a deleted player. `event` may be an UnknownEvent, a
 * type newer than the library: acknowledge it like any other. Deduplicate
 * on `event->id`, since delivery is at least once.
 */
final readonly class RoutedEvent
{
    public function __construct(
        public Event $event,
        public ?string $playerRef,
    ) {}
}
