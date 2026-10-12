// The router's order (ADR 1.10.26af D8, ES3): the signature first, then the
// directory, then the handler, each failure answered with its status.
// Deliveries are signed here with node:crypto at the current time, in the
// Standard Webhooks shape the library verifies.

import { createHmac, randomBytes } from 'node:crypto'
import { readFileSync } from 'node:fs'

import { describe, expect, it } from 'vitest'

import { MemoryPlayerDirectory, type PlayerDirectory } from '../directory.js'
import { HandleFailure, PlayerRouter, type RoutedEvent } from '../router.js'

const repo = new URL('../../../../', import.meta.url)
const vectors: { name: string; body: string }[] = JSON.parse(
  readFileSync(new URL('conformance/vectors/webhook-signatures.json', repo), 'utf8'),
).vectors
const vectorBody = (name: string): string => vectors.find((v) => v.name === name)?.body ?? ''

const key = randomBytes(32)
const secret = `lgr_whsec_${key.toString('base64')}`

/** The headers Lingara sends for `body`, signed now. */
function signed(body: string): Record<string, string> {
  const id = (JSON.parse(body) as { id: string }).id
  const timestamp = String(Math.floor(Date.now() / 1000))
  const signature = createHmac('sha256', key).update(`${id}.${timestamp}.${body}`).digest('base64')
  return { 'webhook-id': id, 'webhook-timestamp': timestamp, 'webhook-signature': `v1,${signature}` }
}

/** A memory directory whose lookups are counted, and which raises when asked to. */
class Counting implements PlayerDirectory {
  readonly inner = new MemoryPlayerDirectory()
  lookups = 0
  constructor(readonly fail = false) {}
  remember(playerRef: string, subject: string): void {
    this.inner.remember(playerRef, subject)
  }
  async playerRefFor(subject: string): Promise<string | undefined> {
    this.lookups += 1
    if (this.fail) throw new Error('the store is down')
    return this.inner.playerRefFor(subject)
  }
  forget(playerRef: string): void {
    this.inner.forget(playerRef)
  }
}

function setup(fail = false) {
  const directory = new Counting(fail)
  directory.remember('p1', 'lgr_sub_conformance')
  const errors: HandleFailure[] = []
  const router = new PlayerRouter({ secrets: secret, directory, onError: (error) => errors.push(error) })
  const seen: RoutedEvent[] = []
  const record = (routed: RoutedEvent): void => {
    seen.push(routed)
  }
  return { directory, errors, router, seen, record }
}

describe('1.10.26af — the router', () => {
  /** 1.10.26af AC20: tampered → 401 with no lookup and no onEvent; valid → the remembered playerRef; unknown subject → null; a raising handler or directory → 500 and onError. */
  it('handle verifies before resolving', async () => {
    const body = vectorBody('valid-single-signature')
    const headers = signed(body)

    const tampered = setup()
    expect(await tampered.router.handle(body.replace('night market', 'day market'), headers, tampered.record)).toBe(401)
    expect(tampered.directory.lookups).toBe(0)
    expect(tampered.seen).toEqual([])
    await expect(tampered.router.route(`${body} `, headers)).rejects.toMatchObject({ reason: 'no_matching_signature' })
    expect(tampered.directory.lookups).toBe(0)

    const notJson = setup()
    const garbage = 'not json'
    const garbageHeaders = { ...headers, 'webhook-signature': `v1,${createHmac('sha256', key).update(`${headers['webhook-id']}.${headers['webhook-timestamp']}.${garbage}`).digest('base64')}` }
    expect(await notJson.router.handle(garbage, garbageHeaders, notJson.record)).toBe(400)
    expect(notJson.directory.lookups).toBe(0)

    const valid = setup()
    expect(await valid.router.handle(new TextEncoder().encode(body), headers, valid.record)).toBe(204)
    expect(valid.seen.map((r) => [r.event.id, r.playerRef])).toEqual([['lgr_evt_conformance1', 'p1']])

    const stranger = setup()
    const unknownBody = body.replace('lgr_sub_conformance', 'lgr_sub_stranger')
    expect(await stranger.router.handle(unknownBody, signed(unknownBody), stranger.record)).toBe(204)
    expect(stranger.seen.map((r) => r.playerRef)).toEqual([null])

    const raising = setup()
    const handlerError = new Error('the handler raised')
    expect(await raising.router.handle(body, headers, () => Promise.reject(handlerError))).toBe(500)
    expect(raising.errors.map((e) => [e.stage, e.cause])).toEqual([['handler', handlerError]])

    const broken = setup(true)
    expect(await broken.router.handle(body, headers, broken.record)).toBe(500)
    expect(broken.seen).toEqual([])
    expect(broken.errors.map((e) => e.stage)).toEqual(['directory'])
    await expect(broken.router.route(body, headers)).rejects.toThrow('the store is down')
  })
})
