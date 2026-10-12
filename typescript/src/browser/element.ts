// `<lingara-embed>`: one shadow-root iframe framing the hosted practice page,
// its attributes, its lifecycle and `reload()`. The class is created inside a
// function so that importing this module never evaluates `extends HTMLElement`
// on a server.

import { checkAttributes, frameSrc, resolveLocale, resolveTheme, type FrameTarget } from './attributes.js'
import { elementErrorDetail, emit } from './events.js'
import { Handshake, type EmbedState, type SessionHost } from './handshake.js'
import type { HostPayloads, Theme } from './protocol.js'
import { TokenKeeper, type TokenProvider } from './tokens.js'

export const DEFAULT_TITLE = 'Lingara practice'
const DEFAULT_HEIGHT = 480
const SANDBOX = 'allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox'

/** The reflected string properties and the attribute each one mirrors. */
export const REFLECTED = {
  surface: 'surface',
  planId: 'plan-id',
  theme: 'theme',
  locale: 'locale',
  lingaraOrigin: 'lingara-origin',
  loading: 'loading',
} as const

const REBUILDING = ['surface', 'plan-id', 'lingara-origin']

// Sizing lives in the shadow stylesheet, never an inline `style`: a `:host`
// rule loses to the host page's own CSS for the element, so its `height` or
// `max-height` always wins.
const sheet = (height: number): string =>
  `:host{display:block;height:${height}px} iframe{border:0;width:100%;height:100%;display:block}`

export interface LingaraEmbedElement extends HTMLElement {
  tokenProvider: TokenProvider | undefined
  readonly state: EmbedState
  reload(): void
  surface: string
  planId: string
  theme: string
  locale: string
  lingaraOrigin: string
  loading: string
}

// The factory's body is the class declaration, so its length is the class's;
// each method stays inside the budget on its own.
// eslint-disable-next-line max-lines-per-function
export function createLingaraEmbedClass(): { new (): LingaraEmbedElement } {
  class LingaraEmbed extends HTMLElement {
    static observedAttributes = ['surface', 'plan-id', 'theme', 'locale', 'lingara-origin', 'title', 'loading']

    private provider: TokenProvider | undefined
    private current: EmbedState = 'idle'
    private live = false
    private frame: HTMLIFrameElement | null = null
    private session: Handshake | null = null
    private sent: { theme?: Theme; locale?: string } = {}
    private readonly styleTag: HTMLStyleElement
    private readonly keeper = new TokenKeeper({
      provider: () => this.provider,
      granted: (token) => this.session?.granted(token),
      failed: (code) => this.session?.providerFailed(code),
    })
    private readonly onMessage = (event: MessageEvent): void => this.session?.onMessage(event)
    private readonly onVisibility = (): void => {
      if (!document.hidden && this.session !== null && !this.session.failed) this.keeper.visible()
    }

    constructor() {
      super()
      this.styleTag = document.createElement('style')
      this.styleTag.textContent = sheet(DEFAULT_HEIGHT)
      this.attachShadow({ mode: 'open' }).append(this.styleTag)
      upgradeProperties(this)
    }

    get tokenProvider(): TokenProvider | undefined {
      return this.provider
    }

    set tokenProvider(provider: TokenProvider | undefined) {
      this.provider = provider
      if (this.live && provider !== undefined && this.current === 'idle') this.start()
    }

    get state(): EmbedState {
      return this.current
    }

    connectedCallback(): void {
      this.live = true
      window.addEventListener('message', this.onMessage)
      document.addEventListener('visibilitychange', this.onVisibility)
      if (this.provider !== undefined) this.start()
    }

    disconnectedCallback(): void {
      this.live = false
      window.removeEventListener('message', this.onMessage)
      document.removeEventListener('visibilitychange', this.onVisibility)
      this.keeper.drop()
      this.session = null
      this.removeFrame()
      this.current = 'idle'
    }

    attributeChangedCallback(name: string, previous: string | null, value: string | null): void {
      if (previous === value) return
      if (name === 'title') this.retitle(value)
      // A rebuilding attribute starts a new session exactly as `reload()` does.
      else if (REBUILDING.includes(name)) this.reload()
      else if (name !== 'loading') this.session?.configure()
    }

    /** A new session: the checks again, a new frame, the back-off reset, a fresh held token reused. */
    reload(): void {
      if (this.live && this.provider !== undefined) this.start()
    }

    private retitle(value: string | null): void {
      if (this.frame !== null) this.frame.title = value ?? DEFAULT_TITLE
    }

    private start(): void {
      this.keeper.halt()
      this.session = null
      const checked = checkAttributes((name) => this.getAttribute(name))
      if (!checked.ok) {
        this.removeFrame()
        this.current = 'failed'
        emit(this, 'lingara:error', elementErrorDetail(checked.code))
        return
      }
      this.session = new Handshake(checked.target, this.keeper, this.host())
      this.keeper.reset()
      this.buildFrame(checked.target)
    }

    private buildFrame(target: FrameTarget): void {
      this.removeFrame()
      this.sent = { theme: resolveTheme(this.getAttribute('theme')), locale: resolveLocale(this.getAttribute('locale')) }
      const frame = document.createElement('iframe')
      frame.title = this.getAttribute('title') ?? DEFAULT_TITLE
      frame.setAttribute('sandbox', SANDBOX)
      frame.setAttribute('allow', '')
      frame.setAttribute('referrerpolicy', 'strict-origin')
      frame.setAttribute('loading', this.getAttribute('loading') === 'eager' ? 'eager' : 'lazy')
      frame.src = frameSrc(target, this.sent.locale, this.sent.theme)
      this.frame = frame
      this.shadowRoot?.append(frame)
      this.session?.frameBuilt()
      this.current = 'connecting'
    }

    private removeFrame(): void {
      this.frame?.remove()
      this.frame = null
    }

    private host(): SessionHost {
      return {
        frameWindow: () => this.frame?.contentWindow ?? null,
        rebuild: () => {
          const checked = checkAttributes((name) => this.getAttribute(name))
          if (checked.ok) this.buildFrame(checked.target)
        },
        setState: (state: EmbedState) => {
          this.current = state
        },
        emit: (name: string, detail: object) => emit(this, name, detail),
        resize: (height: number) => {
          this.styleTag.textContent = sheet(height)
        },
        configureDelta: () => this.configureDelta(),
      }
    }

    private configureDelta(): HostPayloads['configure'] | null {
      const theme = resolveTheme(this.getAttribute('theme'))
      const locale = resolveLocale(this.getAttribute('locale'))
      const delta: HostPayloads['configure'] = {}
      // A theme turned invalid or removed is absent, and absent is the frame's `auto`.
      if (theme !== this.sent.theme) delta.theme = theme ?? 'auto'
      if (locale !== undefined && locale !== this.sent.locale) delta.locale = locale
      this.sent = { theme, locale: locale ?? this.sent.locale }
      return Object.keys(delta).length === 0 ? null : delta
    }
  }
  defineReflected(LingaraEmbed.prototype)
  return LingaraEmbed as unknown as { new (): LingaraEmbedElement }
}

function defineReflected(prototype: HTMLElement): void {
  for (const [property, attribute] of Object.entries(REFLECTED)) {
    Object.defineProperty(prototype, property, {
      configurable: true,
      get(this: HTMLElement) {
        return this.getAttribute(attribute) ?? ''
      },
      set(this: HTMLElement, value: unknown) {
        this.setAttribute(attribute, String(value))
      },
    })
  }
}

/**
 * A property assigned before the upgrade is an own property that shadows the
 * class's accessor: take it off and assign it again through the accessor.
 */
function upgradeProperties(element: HTMLElement): void {
  const record = element as unknown as Record<string, unknown>
  for (const property of ['tokenProvider', ...Object.keys(REFLECTED)]) {
    if (!Object.prototype.hasOwnProperty.call(element, property)) continue
    const value = record[property]
    delete record[property]
    record[property] = value
  }
}
