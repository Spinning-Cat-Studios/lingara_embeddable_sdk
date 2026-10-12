// The fake frame: the iframe's real `contentWindow` with `postMessage` spied,
// frame messages as `MessageEvent`s dispatched on `window` with that `source`
// and `origin`, and a provider whose grants are dated by the (fake) clock.

import { vi, type Mock } from 'vitest'

import { defineLingaraEmbed, EVENTS, type LingaraEmbedElement, type TokenGrant } from '../index.js'

// jsdom creates no browsing context for an iframe inside a shadow root, so
// its `contentWindow` is always `null` there. Each such iframe gets a twin in
// the light DOM (under <html>, out of reach of `body.replaceChildren()`), and
// its `contentWindow` answers with the twin's real window for as long as the
// iframe itself is in a shadow tree. Removed, it answers `null`, as a browser does.
const native = Object.getOwnPropertyDescriptor(HTMLIFrameElement.prototype, 'contentWindow')
const twins = new WeakMap<HTMLIFrameElement, HTMLIFrameElement>()
Object.defineProperty(HTMLIFrameElement.prototype, 'contentWindow', {
  configurable: true,
  get(this: HTMLIFrameElement): Window | null {
    const own = native?.get?.call(this) as Window | null
    if (own !== null || !(this.getRootNode() instanceof ShadowRoot)) return own
    let twin = twins.get(this)
    if (twin === undefined) {
      twin = document.createElement('iframe')
      document.documentElement.append(twin)
      twins.set(this, twin)
    }
    return twin.contentWindow
  },
})

export const ORIGIN = 'https://app.getlingara.com'
export const PLAN = '0b9d4f2e-3c1a-4e8b-9f6d-2a7c5e1b8d40'

export type Provider = Mock<() => Promise<TokenGrant>>

export interface Mounted {
  el: LingaraEmbedElement
  provider: Provider
  /** Every host message posted, to any frame this element built, with its target origin. */
  posts: { message: { type: string; payload: Record<string, unknown> }; target: string }[]
  events: CustomEvent[]
}

export function envelope(type: string, payload: unknown, version = 1): Record<string, unknown> {
  return { protocol: 'lingara-embed', version, type, payload }
}

/** A grant expiring `seconds` from the clock's now. */
export function grant(seconds: number, token = 'lgr_et_first'): TokenGrant {
  return { token, expires_at: new Date(Date.now() + seconds * 1000).toISOString() }
}

/** A provider that answers `grant(seconds)` each call. */
export function granting(seconds = 900): Provider {
  let n = 0
  return vi.fn(async () => grant(seconds, `lgr_et_${(n += 1)}`))
}

export function deferred<T>(): { promise: Promise<T>; resolve(value: T): void; reject(error: unknown): void } {
  let resolve!: (value: T) => void
  let reject!: (error: unknown) => void
  const promise = new Promise<T>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}

export const frameOf = (el: HTMLElement): HTMLIFrameElement | null => el.shadowRoot?.querySelector('iframe') ?? null

const spied = new WeakSet<Window>()

/** Spy on the current frame's `postMessage`, once per window. */
export function track(m: Mounted): void {
  const win = frameOf(m.el)?.contentWindow
  if (win === null || win === undefined || spied.has(win)) return
  spied.add(win)
  vi.spyOn(win, 'postMessage').mockImplementation(((message: Mounted['posts'][number]['message'], target: string) => {
    m.posts.push({ message, target })
  }) as Window['postMessage'])
}

export function mount(attributes: Record<string, string> = { 'plan-id': PLAN }, provider: Provider | null = granting()): Mounted {
  defineLingaraEmbed()
  const el = document.createElement('lingara-embed') as LingaraEmbedElement
  for (const [name, value] of Object.entries(attributes)) el.setAttribute(name, value)
  const m: Mounted = { el, provider: provider ?? vi.fn(), posts: [], events: [] }
  for (const name of Object.values(EVENTS)) el.addEventListener(name, (event) => m.events.push(event as CustomEvent))
  if (provider !== null) el.tokenProvider = provider
  document.body.append(el)
  track(m)
  return m
}

export interface From {
  origin?: string
  source?: Window | null
  version?: number
}

/** The frame posts one message to the host. */
export function frameSays(m: Mounted, type: string, payload: unknown, from: From = {}): void {
  track(m)
  const source = from.source === undefined ? (frameOf(m.el)?.contentWindow ?? null) : from.source
  const data = envelope(type, payload, from.version)
  window.dispatchEvent(new MessageEvent('message', { data, origin: from.origin ?? ORIGIN, source }))
}

export const READY = { versions: [1], surfaces: ['practice'] }

export const sent = (m: Mounted, type?: string): Mounted['posts'] =>
  m.posts.filter((post) => type === undefined || post.message.type === type)

export const fired = (m: Mounted, name: string): CustomEvent[] => m.events.filter((event) => event.type === name)

/** Let settled promises run, and the clock move `ms`. */
export const tick = (ms = 0): Promise<unknown> => vi.advanceTimersByTimeAsync(ms)
