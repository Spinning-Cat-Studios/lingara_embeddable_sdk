// Routing a webhook to its player (ADR 1.10.26af D8): the library's verifier
// on the raw body first, then the directory, so a forged event never reaches
// your store. `handle` maps every outcome to the status all eight kits
// answer Lingara with.

import { Webhook, WebhookVerificationError, type Event, type WebhookHeaders } from '@lingara/api'

import type { PlayerDirectory } from './directory.js'

/** A verified event and the player it concerns, or `null` for a subject the directory does not know (your own, a deleted player). */
export interface RoutedEvent {
  readonly event: Event
  readonly playerRef: string | null
}

/** Where a `500`'s cause goes. It never carries the body, a signature or a secret. */
export type OnError = (error: HandleFailure) => void

export interface PlayerRouterOptions {
  /** Your webhook signing secret (`lgr_whsec_…`), or two during a rotation. */
  secrets: string | readonly string[]
  directory: PlayerDirectory
  /** Default `console.error`. */
  onError?: OnError | undefined
}

/** A failure `handle` answered with `500`: the directory or your handler raised. The original is `cause`. */
export class HandleFailure extends Error {
  readonly stage: 'directory' | 'handler'

  constructor(stage: 'directory' | 'handler', cause: unknown) {
    super(stage === 'directory' ? 'the player directory failed' : 'the event handler failed', { cause })
    this.name = 'HandleFailure'
    this.stage = stage
  }
}

class DirectoryFailure {
  constructor(readonly cause: unknown) {}
}

/** Verifies Lingara's webhooks and resolves each to a player. */
export class PlayerRouter {
  readonly #webhook: Webhook
  readonly #directory: PlayerDirectory
  readonly #onError: OnError

  /** Builds the library's verifier once, so a malformed secret fails here, at startup. */
  constructor(options: PlayerRouterOptions) {
    this.#webhook = new Webhook(options.secrets)
    this.#directory = options.directory
    this.#onError = options.onError ?? ((error) => console.error('lingara webhook:', error))
  }

  /** Verifies the **raw** body, then looks the subject up. Raises the library's `WebhookVerificationError` before any lookup. */
  async route(body: string | Uint8Array, headers: WebhookHeaders): Promise<RoutedEvent> {
    try {
      return await this.#routeMarked(body, headers)
    } catch (error) {
      throw error instanceof DirectoryFailure ? error.cause : error
    }
  }

  /**
   * `route`, then `onEvent`, answered as the status your route sends with an
   * empty body: `204` handled, `401` a bad signature, `400` a malformed
   * payload, `500` the directory or the handler failed (then `onError`).
   */
  async handle(body: string | Uint8Array, headers: WebhookHeaders, onEvent: (routed: RoutedEvent) => unknown): Promise<number> {
    let routed: RoutedEvent
    try {
      routed = await this.#routeMarked(body, headers)
    } catch (error) {
      if (error instanceof WebhookVerificationError) return error.reason === 'malformed_payload' ? 400 : 401
      if (error instanceof DirectoryFailure) return this.#failed(new HandleFailure('directory', error.cause))
      throw error
    }
    try {
      await onEvent(routed)
      return 204
    } catch (error) {
      return this.#failed(new HandleFailure('handler', error))
    }
  }

  /** `route`, with a directory failure told apart from the verifier's. */
  async #routeMarked(body: string | Uint8Array, headers: WebhookHeaders): Promise<RoutedEvent> {
    const event = await this.#webhook.verify(body, headers)
    try {
      const playerRef = await this.#directory.playerRefFor(event.subject)
      return { event, playerRef: playerRef ?? null }
    } catch (error) {
      throw new DirectoryFailure(error)
    }
  }

  #failed(failure: HandleFailure): number {
    try {
      this.#onError(failure)
    } catch {
      // A hook that throws must not turn a 500 into an unhandled rejection.
    }
    return 500
  }

  toJSON(): Record<string, unknown> {
    return { webhook: this.#webhook.toJSON() }
  }

  toString(): string {
    return `PlayerRouter ${JSON.stringify(this.toJSON())}`
  }
}
