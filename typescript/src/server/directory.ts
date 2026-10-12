// The player directory (ADR 1.10.26af D5): the one mapping a kit cannot
// derive. `subject` is a pairwise HMAC computed by Lingara, so the mint's
// answer is the only moment a `playerRef` and its `subject` are in one
// place; the integrator's store keeps them.

/** A value now, or a promise of one: a directory over a database awaits, a map does not. */
export type MaybePromise<T> = T | Promise<T>

/**
 * Your store of `playerRef` ↔ `subject`, unique on `subject` only: a player
 * who links an account owns a second subject beside the guest's. Implement
 * it over your own database, beside your player rows.
 */
export interface PlayerDirectory {
  /** Upserts the row keyed on `subject`. A guest's subject is stable across mints, so remembering it again changes nothing. */
  remember(playerRef: string, subject: string): MaybePromise<void>
  /** The player a subject belongs to, or `null`/`undefined` for one this store never saw (your own subject, a deleted player). */
  playerRefFor(subject: string): MaybePromise<string | null | undefined>
  /** Removes every row for the player. Idempotent. */
  forget(playerRef: string): MaybePromise<void>
}

/**
 * A process-local directory, for tests and single-process demos. It is lost
 * on restart and not shared between instances: production code implements
 * {@link PlayerDirectory} over its own database.
 */
export class MemoryPlayerDirectory implements PlayerDirectory {
  readonly #rows = new Map<string, string>()

  remember(playerRef: string, subject: string): void {
    this.#rows.set(subject, playerRef)
  }

  playerRefFor(subject: string): string | undefined {
    return this.#rows.get(subject)
  }

  forget(playerRef: string): void {
    for (const [subject, owner] of this.#rows) {
      if (owner === playerRef) this.#rows.delete(subject)
    }
  }

  /** Every row, subject → playerRef, sorted by subject. */
  snapshot(): Record<string, string> {
    const sorted = [...this.#rows].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    return Object.fromEntries(sorted)
  }

  toJSON(): Record<string, unknown> {
    return { rows: this.#rows.size }
  }

  toString(): string {
    return `MemoryPlayerDirectory { rows: ${this.#rows.size} }`
  }
}
