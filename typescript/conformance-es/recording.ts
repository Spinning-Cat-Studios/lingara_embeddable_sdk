// What the harness observes beside the kit: a directory decorator that
// counts `playerRefFor` calls (a lookup changes no contents, so only a count
// shows a forged delivery never reached the store), and the log sink that
// ES5's `render` sweeps.

import { MemoryPlayerDirectory, type PlayerDirectory } from '@lingara/embed/server'

/** The case's `MemoryPlayerDirectory`, with its lookups counted. */
export class Recording implements PlayerDirectory {
  readonly #inner = new MemoryPlayerDirectory()
  lookups = 0

  static seeded(seed: Record<string, unknown>): Recording {
    const directory = new Recording()
    for (const [subject, player] of Object.entries(seed)) directory.#inner.remember(String(player), subject)
    return directory
  }

  remember(playerRef: string, subject: string): void {
    this.#inner.remember(playerRef, subject)
  }

  playerRefFor(subject: string): string | undefined {
    this.lookups += 1
    return this.#inner.playerRefFor(subject)
  }

  forget(playerRef: string): void {
    this.#inner.forget(playerRef)
  }

  snapshot(): Record<string, string> {
    return this.#inner.snapshot()
  }

  toJSON(): Record<string, unknown> {
    return { directory: this.#inner.toJSON(), lookups: this.lookups }
  }
}

/** Every line the routers' `onError` and the console wrote. */
export class Sink {
  #lines: string[] = []

  push(line: string): void {
    this.#lines.push(line)
  }

  take(): string[] {
    const lines = this.#lines
    this.#lines = []
    return lines
  }
}

const METHODS = ['debug', 'info', 'log', 'warn', 'error'] as const

/** The console into the sink: the router's default `onError` facility. stdout itself stays the harness's. */
export function captureConsole(sink: Sink): void {
  for (const method of METHODS) {
    console[method] = (...args: unknown[]) => sink.push(`${method} ${args.map(String).join(' ')}`)
  }
}
