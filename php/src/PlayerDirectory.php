<?php

declare(strict_types=1);

namespace Lingara\Embed;

/**
 * The integrator's store of `player_ref` ↔ `subject` (ADR 1.10.26af D5):
 * the one mapping a kit cannot derive, since `subject` is a pairwise HMAC
 * Lingara computes. The mint's answer is the only moment both are in one
 * place, so EmbedServer records it here.
 *
 * Unique on `subject` only: a player who links an account owns a second
 * subject beside the guest's. Implement it over your own database, beside
 * your player rows. Whatever a method throws, the kit propagates unchanged
 * (PlayerRouter::handle() answers it with a 500).
 */
interface PlayerDirectory
{
    /**
     * Upserts the row keyed on `subject`. A guest's subject is stable across
     * mints, so remembering it again changes nothing.
     */
    public function remember(string $playerRef, string $subject): void;

    /**
     * The player a subject belongs to, or null for one this store never saw
     * (your own subject, a deleted player).
     */
    public function playerRefFor(string $subject): ?string;

    /** Removes every row for the player. Idempotent. */
    public function forget(string $playerRef): void;
}
