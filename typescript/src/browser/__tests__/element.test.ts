/** @vitest-environment jsdom */
// The element: its frame, the handshake, attribute changes, errors, reload.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { defineLingaraEmbed, type LingaraEmbedElement, type TokenGrant } from '../index.js'
import { deferred, fired, frameOf, frameSays, grant, granting, mount, ORIGIN, PLAN, READY, sent, tick, track, type Mounted, type Provider } from './fakeFrame.js'

beforeEach(() => {
  vi.useFakeTimers({ now: Date.UTC(2026, 9, 1, 12) })
})
afterEach(() => {
  document.body.replaceChildren()
  vi.useRealTimers()
  vi.restoreAllMocks()
})

async function initialised(m: Mounted): Promise<Mounted> {
  frameSays(m, 'ready', READY)
  await tick()
  return m
}

describe('1.10.26ad — the element', () => {
  // First in the file: nothing has defined the element yet.
  it('1.10.26ad AC15 early properties survive the upgrade and a late grant rebuilds the frame', async () => {
    const provider = granting()
    const early = document.createElement('lingara-embed') as LingaraEmbedElement
    early.tokenProvider = provider
    early.planId = PLAN
    document.body.append(early)
    expect(frameOf(early)).toBeNull()

    expect(defineLingaraEmbed()).toBe(true)
    expect(early.getAttribute('plan-id')).toBe(PLAN)
    const m: Mounted = { el: early, provider, posts: [], events: [] }
    track(m)
    await initialised(m)
    expect(provider).toHaveBeenCalledTimes(1)
    expect(sent(m, 'init')[0].message.payload).toEqual({ token: 'lgr_et_1', plan_id: PLAN })

    for (const [settlesAt, rebuilds] of [[8_500, false], [9_500, true]] as const) {
      document.body.replaceChildren()
      const pending = deferred<TokenGrant>()
      const slow = mount(undefined, vi.fn(() => pending.promise))
      const first = frameOf(slow.el)
      frameSays(slow, 'ready', READY)
      await tick(settlesAt)
      pending.resolve(grant(900))
      await tick()
      expect(frameOf(slow.el) !== first, `${settlesAt} ms`).toBe(rebuilds)
      expect(sent(slow, 'init'), `${settlesAt} ms`).toHaveLength(rebuilds ? 0 : 1)
      if (rebuilds) {
        expect(slow.el.state).toBe('connecting')
        frameSays(slow, 'ready', READY)
        await tick()
        expect(sent(slow, 'init').map((post) => post.message.payload.token)).toEqual(['lgr_et_first'])
      }
      expect(slow.provider).toHaveBeenCalledTimes(1)
    }
  })

  it('1.10.26ad AC2 the frame carries no token and only the sandbox it needs', async () => {
    const m = await initialised(mount({ 'plan-id': PLAN, locale: 'fr', theme: 'dark', title: 'Pratique' }))
    const frame = frameOf(m.el) as HTMLIFrameElement
    expect(m.el.shadowRoot?.querySelectorAll('iframe')).toHaveLength(1)
    expect(frame.src).toBe(`${ORIGIN}/embed/practice?v=1&lng=fr&theme=dark`)
    expect(String(frame.getAttribute('sandbox')).split(' ').sort()).toEqual(
      ['allow-popups', 'allow-popups-to-escape-sandbox', 'allow-same-origin', 'allow-scripts'],
    )
    expect(frame.getAttribute('allow')).toBe('')
    expect(frame.getAttribute('referrerpolicy')).toBe('strict-origin')
    expect(frame.title).toBe('Pratique')
    expect(sent(m, 'init')).toHaveLength(1)
    for (const attribute of frame.getAttributeNames()) {
      expect(frame.getAttribute(attribute), attribute).not.toContain('lgr_et_')
      expect(frame.getAttribute(attribute), attribute).not.toContain(PLAN)
    }
    expect((frameOf(mount().el) as HTMLIFrameElement).src).toBe(`${ORIGIN}/embed/practice?v=1`)
  })

  it('1.10.26ad AC3 the handshake answers only the framed page and coalesces readies', async () => {
    const pending = deferred<TokenGrant>()
    const m = mount(undefined, vi.fn(() => pending.promise))
    for (let i = 0; i < 6; i += 1) frameSays(m, 'ready', READY)
    expect(m.provider).toHaveBeenCalledTimes(1)
    pending.resolve(grant(900))
    await tick()
    expect(sent(m)).toEqual([{ message: expect.objectContaining({ type: 'init', payload: { token: 'lgr_et_first', plan_id: PLAN } }), target: ORIGIN }])

    // A reload of the frame: the held token is reused.
    frameSays(m, 'ready', READY)
    await tick()
    expect(sent(m, 'init')).toHaveLength(2)
    expect(m.provider).toHaveBeenCalledTimes(1)
    expect(m.posts.every((post) => post.target === ORIGIN)).toBe(true)

    // Another origin, another window, a malformed message: nothing at all.
    const other = mount(undefined, granting())
    frameSays(other, 'ready', READY, { origin: 'https://evil.example' })
    frameSays(other, 'ready', READY, { source: window })
    frameSays(other, 'ready', { versions: 'all' })
    await tick()
    expect(other.provider).not.toHaveBeenCalled()
    expect(other.posts).toEqual([])
    expect(other.events).toEqual([])
  })

  it('1.10.26ad AC3 a ready without version 1 or without the surface is refused once', async () => {
    const m = mount()
    for (let i = 0; i < 3; i += 1) frameSays(m, 'ready', { versions: [2], surfaces: ['practice'] }, { version: 2 })
    await tick()
    expect(m.posts).toEqual([])
    expect(m.provider).not.toHaveBeenCalled()
    expect(m.el.state).toBe('failed')
    expect(fired(m, 'lingara:error').map((event) => event.detail)).toEqual([{ code: 'version_unsupported', retryable: false }])

    const n = mount()
    frameSays(n, 'ready', { versions: [1], surfaces: ['dialogue'] })
    frameSays(n, 'ready', { versions: [1], surfaces: ['dialogue'] })
    expect(fired(n, 'lingara:error').map((event) => event.detail)).toEqual([{ code: 'surface_unsupported', retryable: false }])
    expect(n.posts).toEqual([])
  })

  it('1.10.26ad AC4 attributes reconfigure or rebuild and disconnect cleans up', async () => {
    // Before the iframe is built: the URL carries it.
    const idle = mount(undefined, null)
    idle.el.setAttribute('theme', 'light')
    idle.el.tokenProvider = idle.provider
    expect(frameOf(idle.el)?.src).toBe(`${ORIGIN}/embed/practice?v=1&theme=light`)
    expect(idle.posts).toEqual([])

    // Built, not yet initialised: nothing now, one configure after the init.
    const lazy = mount()
    const frame = frameOf(lazy.el)
    lazy.el.setAttribute('theme', 'dark')
    expect(lazy.posts).toEqual([])
    await initialised(lazy)
    expect(lazy.posts.map((post) => post.message.type)).toEqual(['init', 'configure'])
    expect(sent(lazy, 'configure')[0].message.payload).toEqual({ theme: 'dark' })
    expect(frameOf(lazy.el)).toBe(frame)

    // No change: no configure.
    const plain = await initialised(mount())
    expect(plain.posts.map((post) => post.message.type)).toEqual(['init'])

    // After the init: one configure each, the same iframe.
    const same = frameOf(plain.el)
    plain.el.setAttribute('locale', 'ja')
    plain.el.setAttribute('theme', 'light')
    expect(sent(plain, 'configure').map((post) => post.message.payload)).toEqual([{ locale: 'ja' }, { theme: 'light' }])
    expect(frameOf(plain.el)).toBe(same)

    // The three rebuilding attributes replace the iframe.
    const changes: [string, string][] = [['surface', 'practice'], ['plan-id', PLAN.toUpperCase()], ['lingara-origin', 'https://staging.example']]
    for (const [name, value] of changes) {
      const before = frameOf(plain.el)
      plain.el.setAttribute(name, value)
      expect(frameOf(plain.el), name).not.toBe(before)
      expect(frameOf(plain.el), name).not.toBeNull()
    }

    // Removed: no listener, no timer, no provider call.
    document.body.replaceChildren()
    const gone = await initialised(mount())
    const removedFrame = frameOf(gone.el)
    gone.el.remove()
    expect(vi.getTimerCount()).toBe(0)
    window.dispatchEvent(new MessageEvent('message', { data: { protocol: 'lingara-embed', version: 1, type: 'token_request', payload: {} }, origin: ORIGIN, source: removedFrame?.contentWindow ?? null }))
    await tick(3_600_000)
    expect(gone.provider).toHaveBeenCalledTimes(1)
    expect(gone.el.state).toBe('idle')
  })

  it('1.10.26ad AC7 every frame error ends the session and only reload remounts', async () => {
    for (const retryable of [true, false]) {
      document.body.replaceChildren()
      const m = await initialised(mount())
      const frame = frameOf(m.el)
      frameSays(m, 'error', { code: 'rate_limited', retryable })
      expect(m.el.state).toBe('failed')
      expect(frameOf(m.el)).toBe(frame)
      await tick(3_600_000)
      expect(m.provider).toHaveBeenCalledTimes(1)
      frameSays(m, 'ready', READY)
      frameSays(m, 'started', { surface: 'practice', plan_id: PLAN })
      await tick()
      expect(sent(m, 'init')).toHaveLength(1)
      expect(fired(m, 'lingara:error')).toHaveLength(1)
      expect(fired(m, 'lingara:ready')).toHaveLength(0)

      m.el.reload()
      expect(frameOf(m.el)).not.toBe(frame)
      expect(m.el.state).toBe('connecting')
      await initialised(m)
      expect(sent(m, 'init')).toHaveLength(2)
    }
  })

  it('1.10.26ad AC7 reload resets the back-off and does nothing while disconnected', async () => {
    let fail = false
    const provider: Provider = vi.fn(async () => {
      if (fail) throw new Error('down')
      return grant(900)
    })
    const m = await initialised(mount(undefined, provider))
    fail = true
    frameSays(m, 'token_request', { reason: 'expired' })
    await tick(2_000)
    expect(provider).toHaveBeenCalledTimes(3)
    frameSays(m, 'error', { code: 'token_expired', retryable: true })

    m.el.reload()
    await initialised(m)
    frameSays(m, 'token_request', { reason: 'expired' })
    await tick()
    expect(provider).toHaveBeenCalledTimes(4)
    // The first wait again, not the third.
    await tick(2_000)
    expect(provider).toHaveBeenCalledTimes(5)

    m.el.remove()
    m.el.reload()
    expect(frameOf(m.el)).toBeNull()
    expect(m.el.state).toBe('idle')
  })
})
