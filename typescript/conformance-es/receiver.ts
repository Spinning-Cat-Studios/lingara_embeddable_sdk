// The webhook receiver the conformance server posts to, on node:http.
// Delivery `n` of the armed case is answered through a `PlayerRouter` built
// from that item's `router.secrets`, whose `onEvent` records what it saw and
// raises when the item's `handler` is `raise`.

import type { IncomingMessage, ServerResponse } from 'node:http'

import { UnknownEvent } from '@lingara/api'
import { PlayerRouter, type RoutedEvent } from '@lingara/embed/server'

import type { Recording, Sink } from './recording.js'

/** What one delivery's `onEvent` saw. */
export interface Seen {
  playerRef: string | null
  eventId: string
  unknown: boolean
}

export type Json = Record<string, unknown>

/** The armed case, as the receiver needs it. */
export interface Armed {
  deliver: Json[]
  directory: Recording
  seen: (Seen | null)[]
  received: number
  answered: number
}

export class Receiver {
  armed: Armed | null = null

  /** `secrets`: `primary` and `secondary`, from `LINGARA_EMBED_WEBHOOK_SECRETS`. */
  constructor(
    readonly secrets: Map<string, string>,
    readonly sink: Sink,
  ) {}

  /** The router delivery `item` is answered through. */
  router(item: Json, directory: Recording): PlayerRouter {
    const named = (item['router'] as { secrets?: string[] } | undefined)?.secrets ?? ['primary']
    const secrets = named.map((name) => {
      const secret = this.secrets.get(name)
      if (secret === undefined) throw new Error(`no secret named ${name}`)
      return secret
    })
    return new PlayerRouter({ secrets, directory, onError: (failure) => this.sink.push(`on_error: ${String(failure)}`) })
  }

  /** The node:http listener. */
  readonly listener = (request: IncomingMessage, response: ServerResponse): void => {
    const chunks: Buffer[] = []
    request.on('data', (chunk: Buffer) => chunks.push(chunk))
    request.on('end', () => {
      void this.#deliver(new Uint8Array(Buffer.concat(chunks)), request.headers).then((status) => {
        response.writeHead(status).end()
      })
    })
  }

  async #deliver(body: Uint8Array, headers: IncomingMessage['headers']): Promise<number> {
    const armed = this.armed
    if (armed === null) return 409
    const n = armed.received
    armed.received += 1
    const item = armed.deliver[n] ?? {}
    let status: number
    try {
      const raise = item['handler'] === 'raise'
      status = await this.router(item, armed.directory).handle(body, headers, (routed) => this.#record(armed, n, routed, raise))
    } catch (error) {
      this.sink.push(`harness: ${String(error)}`)
      status = 500
    }
    armed.answered += 1
    return status
  }

  #record(armed: Armed, n: number, routed: RoutedEvent, raise: boolean): void {
    armed.seen[n] = { playerRef: routed.playerRef, eventId: routed.event.id, unknown: routed.event instanceof UnknownEvent }
    if (raise) throw new Error('the conformance handler raised, as the case asked')
  }
}
