package com.getlingara.embed.server.kotlin

import java.util.TreeMap

/**
 * The integrator's store of `player_ref` and `subject` (ADR 1.10.26af D5): the one mapping a kit
 * cannot derive, because `subject` is a pairwise HMAC Lingara computes. The mint's answer is the
 * only moment both are in one place, so [EmbedServer] records it here.
 *
 * Implement it over your own database, beside your player rows: two columns, `player_ref` and
 * `subject`, with a unique index on `subject` and a plain one on `player_ref`. One player may own
 * more than one subject (a guest's, and a linked account's). Whatever a method throws, the kit
 * propagates unchanged.
 */
public interface PlayerDirectory {
    /** Upserts the row keyed on [subject]. A guest's subject is stable across mints. */
    public suspend fun remember(
        playerRef: String,
        subject: String,
    )

    /** The player [subject] belongs to, or null for one this store never saw. */
    public suspend fun playerRefFor(subject: String): String?

    /** Removes every row for the player. Idempotent. */
    public suspend fun forget(playerRef: String)
}

/**
 * A process-local [PlayerDirectory], for tests and single-process demos. It is lost on restart and
 * not shared between instances: production code implements [PlayerDirectory] over its own
 * database. It is safe for concurrent use.
 */
public class MemoryPlayerDirectory : PlayerDirectory {
    private val rows = TreeMap<String, String>()

    override suspend fun remember(
        playerRef: String,
        subject: String,
    ) {
        synchronized(rows) { rows[subject] = playerRef }
    }

    override suspend fun playerRefFor(subject: String): String? = synchronized(rows) { rows[subject] }

    override suspend fun forget(playerRef: String) {
        synchronized(rows) { rows.values.removeIf { it == playerRef } }
    }

    /** Every row, subject to player_ref, sorted by subject. */
    public fun snapshot(): Map<String, String> = synchronized(rows) { TreeMap(rows) }

    override fun toString(): String = "MemoryPlayerDirectory(rows=${synchronized(rows) { rows.size }})"
}
