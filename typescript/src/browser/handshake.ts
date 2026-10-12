// One session's half of the protocol: which messages are heard, how `ready`s
// coalesce onto one grant, when a late grant rebuilds the frame, and that a
// frame `error` is terminal. The element owns the DOM; this owns the decisions.

import type { FrameTarget } from './attributes.js'
import {
  clampHeight,
  completedDetail,
  elementErrorDetail,
  EVENTS,
  frameErrorDetail,
  readyDetail,
  resizeDetail,
  type ElementCode,
  type ErrorDetail,
} from './events.js'
import { hostMessage, parseFrameMessage, VERSION, type FrameMessage, type FramePayloads, type HostPayloads, type HostType } from './protocol.js'
import type { TokenKeeper } from './tokens.js'

export type EmbedState = 'idle' | 'connecting' | 'ready' | 'failed'

/**
 * The frame stops waiting for `init` 10 s after its load; a grant that
 * settles later than this after the first `ready` it answers rebuilds the
 * frame instead. The 1 s margin covers the load-to-first-`ready` gap.
 */
export const LATE_GRANT_MS = 9_000

/** What a session needs from its element. */
export interface SessionHost {
  frameWindow(): Window | null
  rebuild(): void
  setState(state: EmbedState): void
  emit(name: string, detail: object): void
  resize(height: number): void
  /** The theme and locale that differ from what this frame was last told; `null` when neither does. */
  configureDelta(): HostPayloads['configure'] | null
}

export class Handshake {
  failed = false
  private initSent = false
  private awaitingInit = false
  private firstReadyAt: number | undefined

  constructor(
    private readonly target: FrameTarget,
    private readonly keeper: TokenKeeper,
    private readonly host: SessionHost,
  ) {}

  /** A new iframe: nothing has been sent to it yet. */
  frameBuilt(): void {
    this.initSent = false
    this.awaitingInit = false
    this.firstReadyAt = undefined
  }

  /** Whether the current frame has been sent an `init`, so `token` and `configure` may follow. */
  get initialised(): boolean {
    return this.initSent
  }

  /** The window `message` listener: only the framed page, only parseable, only while live. */
  onMessage(event: MessageEvent): void {
    if (this.failed || event.origin !== this.target.origin) return
    const source = this.host.frameWindow()
    if (source === null || event.source !== source) return
    const message = parseFrameMessage(event.data)
    if (message !== null) this.handle(message)
  }

  /** The keeper's `granted` hook. */
  granted(token: string): void {
    if (this.failed) return
    if (!this.awaitingInit) {
      if (this.initSent) this.post('token', { token })
      return
    }
    if (this.firstReadyAt !== undefined && Date.now() - this.firstReadyAt > LATE_GRANT_MS) {
      // The frame stopped waiting; the keeper holds the grant for the new frame's first `ready`.
      this.host.rebuild()
      return
    }
    this.sendInit(token)
  }

  /** The keeper's `failed` hook. */
  providerFailed(code: 'token_provider_invalid' | 'token_provider_failed'): void {
    if (this.failed) return
    // Before the frame has its `init`, no frame session exists and none will ask.
    if (code === 'token_provider_failed' && (!this.initSent || this.awaitingInit)) {
      this.fail(elementErrorDetail(code))
      return
    }
    this.host.emit(EVENTS.error, elementErrorDetail(code))
  }

  /** A `theme` or `locale` change: told to a frame that has its `init`, else carried by the next one. */
  configure(): void {
    if (this.failed || !this.initSent) return
    const delta = this.host.configureDelta()
    if (delta !== null) this.post('configure', delta)
  }

  /** Ends the session: no more messages heard, no more provider calls. */
  fail(detail: ErrorDetail): void {
    this.failed = true
    this.keeper.halt()
    this.host.setState('failed')
    this.host.emit(EVENTS.error, detail)
  }

  private handle(message: FrameMessage): void {
    switch (message.type) {
      case 'ready':
        return this.onReady(message.payload)
      case 'started':
        this.host.setState('ready')
        return this.host.emit(EVENTS.ready, readyDetail(message.payload))
      case 'token_request':
        return this.keeper.request()
      case 'resized': {
        const height = clampHeight(message.payload.height)
        this.host.resize(height)
        return this.host.emit(EVENTS.resize, resizeDetail(height))
      }
      case 'practice_completed':
        return this.host.emit(EVENTS.practiceCompleted, completedDetail(message.payload))
      case 'error':
        return this.fail(frameErrorDetail(message.payload))
    }
  }

  private onReady(payload: FramePayloads['ready']): void {
    const refusal = this.refuse(payload)
    if (refusal !== null) return this.fail(elementErrorDetail(refusal))
    // A `ready` after an `init` is a frame that reloaded itself.
    this.host.setState('connecting')
    this.firstReadyAt ??= Date.now()
    if (this.keeper.busy()) {
      this.awaitingInit = true
      return
    }
    const token = this.keeper.fresh()
    if (token !== undefined) return this.sendInit(token)
    this.awaitingInit = true
    this.keeper.request()
  }

  private refuse(payload: FramePayloads['ready']): ElementCode | null {
    if (!payload.versions.includes(VERSION)) return 'version_unsupported'
    if (!payload.surfaces.includes(this.target.surface)) return 'surface_unsupported'
    return null
  }

  private sendInit(token: string): void {
    this.post('init', { token, plan_id: this.target.planId })
    this.initSent = true
    this.awaitingInit = false
    this.firstReadyAt = undefined
    this.configure()
  }

  private post<T extends HostType>(type: T, payload: HostPayloads[T]): void {
    this.host.frameWindow()?.postMessage(hostMessage(type, payload), this.target.origin)
  }
}
