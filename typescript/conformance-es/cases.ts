// One ES case, start to finish (ES `CONTRACT.md`, *What a harness reports*):
// fetch, arm, the `kit:` jobs, every delivery, the caller's comparisons,
// ES5's sweep and the finish, merged into one result line.

import { inspect } from 'node:util'

import { Lingara } from '@lingara/api'
import { EmbedServer } from '@lingara/embed/server'

import { judge, run, type Outcome } from './kit.js'
import type { Json, Receiver, Seen } from './receiver.js'
import { Recording } from './recording.js'

/** The language this harness reports: `make/typescript.mk`'s `ES_LANGS` entry. */
const LANG = 'typescript'
const SECRET_PREFIXES = ['lgr_et_', 'lgr_cs_', 'lgr_whsec_']

export function env(name: string): string {
  const value = process.env[name]
  if (value === undefined) throw new Error(`${name} is not set`)
  return value
}

/** `object[key]`, or `fallback` when absent. */
function field<T>(object: Json | undefined, key: string, fallback: T): T {
  return (object?.[key] as T | undefined) ?? fallback
}

/** A control call's answer, or why there was none. */
async function control(url: string, init?: RequestInit): Promise<Response | string> {
  try {
    const response = await fetch(url, init)
    return response.ok ? response : `status ${response.status}`
  } catch (error) {
    return String(error)
  }
}

const sleep = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms))

/** Until every delivery has been answered, or 15 s; then what each saw. */
async function waitForDeliveries(receiver: Receiver, count: number): Promise<(Seen | null)[]> {
  const deadline = Date.now() + 15_000
  while ((receiver.armed?.answered ?? count) < count && Date.now() < deadline) await sleep(20)
  return receiver.armed?.seen ?? []
}

/** Whether one delivery's `onEvent` saw what its `expect.handler` names. */
function handlerMatches(want: unknown, got: Seen | null): boolean {
  if (want === 'not_called') return got === null
  if (got === null) return false
  return typeof want === 'object' && want !== null ? seenMatches(want as Json, got) : want === undefined
}

/** Each key `expect.handler` names, against what was seen. */
function seenMatches(want: Json, got: Seen): boolean {
  const player = !('player_ref' in want) || want['player_ref'] === got.playerRef
  const event = typeof want['event_id'] !== 'string' || want['event_id'] === got.eventId
  const unknown = typeof want['unknown'] !== 'boolean' || want['unknown'] === got.unknown
  return player && event && unknown
}

/** Each delivery's `expect.handler` against what its `onEvent` saw. */
function handlerMismatches(id: string, deliver: Json[], seen: (Seen | null)[]): string[] {
  return deliver.flatMap((item, n) => {
    const want = (item['expect'] as Json | undefined)?.['handler']
    const got = seen[n] ?? null
    return handlerMatches(want, got) ? [] : [`${id}: fail handler: deliver[${n}] saw ${JSON.stringify(got)}, expected ${JSON.stringify(want)}`]
  })
}

/** ES5: no rendering of a kit object, and no captured log line, carries a secret prefix. */
function renderMismatches(id: string, receiver: Receiver, embed: EmbedServer, directory: Recording, out: Outcome): string[] {
  const router = receiver.router({ router: { secrets: ['primary', 'secondary'] } }, directory)
  const objects: unknown[] = [embed, router, directory, out.minted]
  const renderings = objects.flatMap((o) => [String(o), JSON.stringify(o) ?? '', inspect(o, { depth: 8, showHidden: true })])
  renderings.push(...receiver.sink.take())
  const leaks = renderings.filter((r) => SECRET_PREFIXES.some((prefix) => r.includes(prefix)))
  return leaks.length === 0 ? [] : [`${id}: fail render: ${leaks.length} rendering(s) carry a secret prefix`]
}

/** One case, start to finish: its result line. */
export async function runCase(receiver: Receiver, id: string): Promise<Json> {
  const server = env('LINGARA_EMBED_SERVER_URL')
  const url = (verb: string): string => `${server}/__conformance/${verb}/${id}`
  const fetched = await control(url('cases'))
  if (typeof fetched === 'string') return { case: id, lang: LANG, result: 'fail', mismatches: [`cannot read the case: ${fetched}`] }
  const c = (await fetched.json()) as Json
  const directory = Recording.seeded(field(field<Json | undefined>(c, 'directory', undefined), 'seed', {}))
  const deliver = field<Json[]>(c, 'deliver', [])
  receiver.armed = { deliver, directory, seen: deliver.map(() => null), received: 0, answered: 0 }
  receiver.sink.take()
  const client = new Lingara({ clientId: env('LINGARA_EMBED_CLIENT_ID'), clientSecret: env('LINGARA_EMBED_CLIENT_SECRET'), baseUrl: server, tokenUrl: `${server}/oauth/token` })
  const embed = new EmbedServer(client, { directory })

  const mismatches: string[] = []
  const armed = await control(url('arm'), { method: 'POST' })
  if (typeof armed === 'string') mismatches.push(`cannot arm: ${armed}`)
  const outcome = await run(embed, field<Json[]>(c, 'kit', []))
  const seen = await waitForDeliveries(receiver, deliver.length)
  mismatches.push(...judge(id, field(c, 'expect', {}), outcome, directory), ...handlerMismatches(id, deliver, seen))
  if (outcome.render) mismatches.push(...renderMismatches(id, receiver, embed, directory, outcome))
  mismatches.push(...(await finish(url('finish'), seen)))
  receiver.armed = null
  return { case: id, lang: LANG, result: mismatches.length === 0 ? 'pass' : 'fail', mismatches }
}

/** `POST finish` with each delivery's player, in order; the server's own mismatches. */
async function finish(url: string, seen: (Seen | null)[]): Promise<string[]> {
  const handlers = seen.map((s) => s?.playerRef ?? null)
  try {
    const response = await fetch(url, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ handlers }) })
    const body = (await response.json()) as { mismatches?: string[] }
    return body.mismatches ?? []
  } catch (error) {
    return [`cannot finish: ${String(error)}`]
  }
}

