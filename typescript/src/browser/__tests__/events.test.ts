/** @vitest-environment jsdom */
// Frame messages become DOM events with fixed details.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { ELEMENT_CODES, ERROR_CODES } from '../index.js'
import { fired, frameOf, frameSays, mount, PLAN, READY, tick, type Mounted } from './fakeFrame.js'

beforeEach(() => {
  vi.useFakeTimers()
})
afterEach(() => {
  document.body.replaceChildren()
  vi.useRealTimers()
})

async function started(): Promise<Mounted> {
  const m = mount()
  frameSays(m, 'ready', READY)
  await tick()
  frameSays(m, 'started', { surface: 'practice', plan_id: PLAN })
  return m
}

const hostRule = (m: Mounted): string => m.el.shadowRoot?.querySelector('style')?.textContent ?? ''

describe('1.10.26ad — DOM events', () => {
  it('1.10.26ad AC6 frame messages become DOM events with fixed details', async () => {
    const m = await started()
    const completed = { plan_id: PLAN, set_number: 2, correct: 3, total: 4, event_id: 'evt_01', extra: 'no' }
    frameSays(m, 'practice_completed', completed)
    frameSays(m, 'practice_completed', { ...completed, event_id: null })

    const [ready] = fired(m, 'lingara:ready')
    expect(ready.detail).toStrictEqual({ surface: 'practice', planId: PLAN })
    expect(m.el.state).toBe('ready')
    const done = fired(m, 'lingara:practice-completed').map((event) => event.detail)
    expect(done[0]).toStrictEqual({ planId: PLAN, setNumber: 2, correct: 3, total: 4, eventId: 'evt_01' })
    expect(done[1].eventId).toBeNull()
    for (const event of m.events) {
      expect(event.bubbles && event.composed, event.type).toBe(true)
    }
  })

  it('1.10.26ad AC6 a resized height is clamped into the shadow stylesheet', async () => {
    const m = await started()
    for (const [height, clamped] of [[50, 120], [612, 612], [9000, 4000]]) {
      frameSays(m, 'resized', { height })
      expect(hostRule(m)).toContain(`:host{display:block;height:${clamped}px}`)
      expect(fired(m, 'lingara:resize').at(-1)?.detail).toStrictEqual({ height: clamped })
    }
    expect(hostRule(m)).toContain('iframe{border:0;width:100%;height:100%;display:block}')
    expect(m.el.getAttribute('style')).toBeNull()
    expect(frameOf(m.el)?.getAttribute('style')).toBeNull()
  })

  it('1.10.26ad AC6 frame error codes pass through and unknown ones become internal', async () => {
    for (const code of ERROR_CODES) {
      const m = await started()
      frameSays(m, 'error', { code, retryable: true, extra: 1 })
      expect(fired(m, 'lingara:error').map((event) => event.detail)).toStrictEqual([{ code, retryable: true }])
    }
    for (const code of ['token_provider_failed', 'lingara_origin_invalid', 'brand_new']) {
      const m = await started()
      frameSays(m, 'error', { code, retryable: true })
      expect(fired(m, 'lingara:error').map((event) => event.detail)).toStrictEqual([{ code: 'internal', retryable: false }])
    }
  })

  it("1.10.26ad AC6 the element's own codes carry their retryable", () => {
    expect(ELEMENT_CODES).toStrictEqual({
      surface_unsupported: false,
      plan_not_found: false,
      version_unsupported: false,
      lingara_origin_invalid: false,
      token_provider_invalid: true,
      token_provider_failed: true,
    })
  })
})
