/** @vitest-environment jsdom */
// The three attributes checked before a frame is built.

import { afterEach, describe, expect, it } from 'vitest'

import { normaliseOrigin } from '../attributes.js'
import { fired, frameOf, mount, PLAN } from './fakeFrame.js'

afterEach(() => document.body.replaceChildren())

function refused(attributes: Record<string, string>, code: string): void {
  const m = mount(attributes)
  expect(frameOf(m.el), JSON.stringify(attributes)).toBeNull()
  expect(m.provider).not.toHaveBeenCalled()
  expect(m.el.state).toBe('failed')
  expect(fired(m, 'lingara:error').map((event) => event.detail)).toEqual([{ code, retryable: false }])
}

function framed(attributes: Record<string, string>): string {
  const m = mount(attributes)
  const frame = frameOf(m.el)
  expect(frame, JSON.stringify(attributes)).not.toBeNull()
  expect(m.el.state).toBe('connecting')
  return frame?.src ?? ''
}

describe('1.10.26ad — attributes', () => {
  it('1.10.26ad AC8 only valid origins surfaces and UUID plan ids are framed', () => {
    for (const origin of ['https://staging.example', 'http://localhost:5187', 'http://127.0.0.1']) {
      expect(normaliseOrigin(origin), origin).toBe(origin)
      expect(framed({ 'plan-id': PLAN, 'lingara-origin': origin })).toBe(`${origin}/embed/practice?v=1`)
    }
    for (const origin of ['https://App.Example/', 'https://app.example:443']) {
      expect(normaliseOrigin(origin)).toBe('https://app.example')
      expect(framed({ 'plan-id': PLAN, 'lingara-origin': origin })).toBe('https://app.example/embed/practice?v=1')
    }
    for (const origin of ['http://example.com', 'javascript:', 'https://x/path', 'https://u@x', 'http://[::1]', '', 'https://x?', 'https://x/#a']) {
      expect(normaliseOrigin(origin), origin).toBeNull()
      refused({ 'plan-id': PLAN, 'lingara-origin': origin }, 'lingara_origin_invalid')
    }

    for (const plan of [PLAN, PLAN.toUpperCase(), '00000000-0000-0000-0000-000000000000']) framed({ 'plan-id': plan })
    refused({}, 'plan_not_found')
    for (const plan of ['plan-1', PLAN.replace(/-/g, ''), `{${PLAN}}`]) refused({ 'plan-id': plan }, 'plan_not_found')

    expect(framed({ 'plan-id': PLAN, surface: '' })).toBe('https://app.getlingara.com/embed/practice?v=1')
    for (const surface of ['dialogue', '../x']) refused({ 'plan-id': PLAN, surface }, 'surface_unsupported')

    // With no provider there is no session yet, so even an absent plan id is silent.
    const quiet = mount({}, null)
    expect(quiet.events).toEqual([])
    expect(quiet.el.state).toBe('idle')
    expect(frameOf(quiet.el)).toBeNull()
  })
})
