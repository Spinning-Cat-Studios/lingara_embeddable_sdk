/** @vitest-environment jsdom */
// One clock re-mints before expiry and backs off; the grant validator.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import notices from '../../../../conformance/vectors/update-notice.json'
import { updateLine, type UpdateAvailableDetail } from '../events.js'
import { validateGrant } from '../tokens.js'
import { deferred, fired, frameSays, grant, mount, READY, sent, tick, type Mounted, type Provider } from './fakeFrame.js'
import fixtures from './grants.json'

beforeEach(() => {
  vi.useFakeTimers({ now: Date.UTC(2026, 9, 1, 12) })
})
afterEach(() => {
  document.body.replaceChildren()
  vi.useRealTimers()
  vi.restoreAllMocks()
})

/** A provider whose n-th answer is `answers[n]`: a grant lifetime in seconds, a raw answer, or `'reject'`. */
function scripted(answers: (number | 'reject' | object)[]): Provider {
  let n = 0
  return vi.fn(async () => {
    const answer = answers[Math.min(n, answers.length - 1)]
    n += 1
    if (answer === 'reject') throw new Error('endpoint down')
    return typeof answer === 'number' ? grant(answer, `lgr_et_${n}`) : (answer as never)
  })
}

async function initialised(provider: Provider): Promise<Mounted> {
  const m = mount(undefined, provider)
  frameSays(m, 'ready', READY)
  await tick()
  expect(sent(m, 'init')).toHaveLength(1)
  return m
}

describe('1.10.26ad — tokens', () => {
  it('1.10.26ad AC5 one clock re-mints before expiry and backs off', async () => {
    // 900 s → 840 s, and the re-mint is posted as `token`.
    const long = await initialised(scripted([900]))
    await tick(839_999)
    expect(long.provider).toHaveBeenCalledTimes(1)
    await tick(1)
    expect(long.provider).toHaveBeenCalledTimes(2)
    expect(sent(long, 'token').map((post) => post.message.payload)).toEqual([{ token: 'lgr_et_2' }])
    document.body.replaceChildren()

    // 45 s, and already past: both posted, both re-minted at the 30 s floor.
    for (const lifetime of [45, -600]) {
      const short = await initialised(scripted([lifetime]))
      expect(sent(short, 'init')[0].message.payload.token).toBe('lgr_et_1')
      await tick(29_999)
      expect(short.provider).toHaveBeenCalledTimes(1)
      await tick(1)
      expect(short.provider).toHaveBeenCalledTimes(2)
      document.body.replaceChildren()
    }

    // Year 9999: the 840 s ceiling, never a tight loop or an overflowed timer.
    const far = await initialised(scripted([{ token: 'lgr_et_far', expires_at: '9999-12-31T23:59:59Z' }]))
    await tick(839_999)
    expect(far.provider).toHaveBeenCalledTimes(1)
    await tick(1)
    expect(far.provider).toHaveBeenCalledTimes(2)
  })

  it('1.10.26ad AC5 concurrent triggers share one provider call', async () => {
    const pending = deferred<{ token: string; expires_at: string }>()
    const provider: Provider = vi.fn(() => pending.promise)
    const m = mount(undefined, provider)
    frameSays(m, 'ready', READY)
    frameSays(m, 'token_request', { reason: 'expired' })
    expect(provider).toHaveBeenCalledTimes(1)
    pending.resolve(grant(900))
    await tick()
    expect(sent(m, 'init')).toHaveLength(1)
  })

  it('1.10.26ad AC5 after an init, failures back off 2 s, 10 s and 40 s, then report', async () => {
    const m = await initialised(scripted([900, 'reject']))
    frameSays(m, 'token_request', { reason: 'expired' })
    await tick()
    expect(m.provider).toHaveBeenCalledTimes(2)
    for (const [wait, calls] of [[2_000, 3], [8_000, 4], [30_000, 5]]) {
      await tick(wait - 1)
      expect(m.provider).toHaveBeenCalledTimes(calls - 1)
      await tick(1)
      expect(m.provider).toHaveBeenCalledTimes(calls)
    }
    expect(fired(m, 'lingara:error').map((event) => event.detail)).toEqual([{ code: 'token_provider_failed', retryable: true }])
    expect(m.el.state).toBe('connecting')

    // Scheduling stopped; the next ask gets exactly one attempt and no second event.
    await tick(900_000)
    expect(m.provider).toHaveBeenCalledTimes(5)
    frameSays(m, 'token_request', { reason: 'expired' })
    await tick(60_000)
    expect(m.provider).toHaveBeenCalledTimes(6)
    expect(fired(m, 'lingara:error')).toHaveLength(1)
  })

  it('1.10.26ad AC5 before an init, the same failures end the session and readies wait', async () => {
    const m = mount(undefined, scripted(['reject']))
    frameSays(m, 'ready', READY)
    await tick()
    for (let t = 0; t < 40_000; t += 500) {
      frameSays(m, 'ready', READY)
      await tick(500)
    }
    expect(m.provider).toHaveBeenCalledTimes(4)
    expect(m.el.state).toBe('failed')
    expect(sent(m)).toEqual([])
    expect(fired(m, 'lingara:error').map((event) => event.detail)).toEqual([{ code: 'token_provider_failed', retryable: true }])
  })

  it('1.10.26ad AC5 a token_request during back-off calls at once', async () => {
    const m = await initialised(scripted([900, 'reject', 900]))
    frameSays(m, 'token_request', { reason: 'expired' })
    await tick(500)
    expect(m.provider).toHaveBeenCalledTimes(2)
    frameSays(m, 'token_request', { reason: 'expired' })
    await tick()
    expect(m.provider).toHaveBeenCalledTimes(3)
    expect(sent(m, 'token')).toHaveLength(1)
  })

  it('1.10.26ad AC5 a tab back in view re-mints a token with 60 s or less left', async () => {
    const m = await initialised(scripted([900]))
    const visibility = vi.spyOn(document, 'hidden', 'get').mockReturnValue(true)
    const start = Date.now()
    // A background tab's timer is throttled: the clock moves, the timer has not fired.
    vi.setSystemTime(start + 800_000)
    visibility.mockReturnValue(false)
    document.dispatchEvent(new Event('visibilitychange'))
    expect(m.provider).toHaveBeenCalledTimes(1)
    vi.setSystemTime(start + 845_000)
    document.dispatchEvent(new Event('visibilitychange'))
    expect(m.provider).toHaveBeenCalledTimes(2)
    await tick()
    expect(sent(m, 'token')).toHaveLength(1)
  })

  it('1.10.26ad AC5 an invalid grant is reported once and counts as a failure', async () => {
    const m = await initialised(scripted([900, { token: 'et_bad', expires_at: '2026-10-01T13:00:00Z' }]))
    frameSays(m, 'token_request', { reason: 'expired' })
    await tick()
    await tick(2_000)
    expect(m.provider).toHaveBeenCalledTimes(3)
    expect(fired(m, 'lingara:error').map((event) => event.detail)).toEqual([{ code: 'token_provider_invalid', retryable: true }])
  })

  it('1.10.26ad AC5 the token never reaches storage, an attribute, a detail or the console', async () => {
    const logs = (['log', 'info', 'warn', 'error', 'debug'] as const).map((level) => vi.spyOn(console, level))
    const setItem = vi.spyOn(Storage.prototype, 'setItem')
    const m = await initialised(scripted([900]))
    frameSays(m, 'started', { surface: 'practice', plan_id: m.el.getAttribute('plan-id') })
    frameSays(m, 'resized', { height: 400 })
    frameSays(m, 'error', { code: 'token_expired', retryable: true })

    const surfaces = [
      m.el.outerHTML,
      m.el.shadowRoot?.innerHTML ?? '',
      JSON.stringify(m.events.map((event) => event.detail)),
      document.cookie,
    ]
    for (const text of surfaces) expect(text).not.toContain('lgr_et_')
    expect(setItem).not.toHaveBeenCalled()
    for (const log of logs) expect(log).not.toHaveBeenCalled()
  })

  it('30.10.26p AC9 an update notice reaches the host once', async () => {
    // The core's own lines: every row without an embed_version is the widget's.
    const rows = notices.rows.filter((row) => row.embed_version === null)
    expect(rows.length).toBeGreaterThan(8)
    for (const row of rows) expect(updateLine(row.update_available as UpdateAvailableDetail), row.case).toBe(row.line)

    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    const notice = { latest: '1.2.0', deprecated: true, message: 'please update' }
    const bad = { latest: '1.2.0', message: null }
    const with_ = (n: unknown) => ({ ...grant(900, 'lgr_et_n'), update_available: n })
    const m = await initialised(scripted([with_(notice), with_(notice), 900, with_(bad), with_(notice)]))
    for (let i = 0; i < 3; i += 1) {
      frameSays(m, 'token_request', { reason: 'expired' })
      await tick()
    }
    expect(m.provider).toHaveBeenCalledTimes(4)
    expect(sent(m, 'token')).toHaveLength(3)
    expect(fired(m, 'lingara:update-available').map((event) => event.detail)).toEqual([notice])
    expect(warn.mock.calls).toEqual([["lingara: this build's embed version is deprecated; the current version is 1.2.0: please update"]])
    // The frame hears tokens and nothing else.
    for (const post of m.posts) expect(Object.keys(post.message.payload)).not.toContain('update_available')
    // A malformed notice still grants.
    expect(validateGrant(with_(bad))).toEqual({ token: 'lgr_et_n', expiresAt: expect.any(Number) })

    // A re-connected element is a new session, and is told again.
    m.el.remove()
    document.body.append(m.el)
    frameSays(m, 'ready', READY)
    await tick()
    expect(m.provider).toHaveBeenCalledTimes(5)
    expect(fired(m, 'lingara:update-available')).toHaveLength(2)
    expect(warn).toHaveBeenCalledTimes(2)
  })

  it('1.10.26ad AC14 every handoff shape V1 allows is read the same way', () => {
    for (const { name, answer, expect: expected } of fixtures.cases) {
      const parsed = validateGrant(answer)
      if ('invalid' in expected) expect(parsed, name).toBeNull()
      else expect(parsed, name).toEqual({ token: 'lgr_et_a', expiresAt: expected.expiresAtMs })
    }
  })
})
