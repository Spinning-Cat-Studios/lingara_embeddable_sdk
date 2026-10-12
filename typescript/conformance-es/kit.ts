// A case's `kit:` jobs through `EmbedServer`, and the comparisons only the
// kit's caller can make (ES `CONTRACT.md`, *What a harness reports*).

import { ApiError, MaintenanceError, OAuthError, TransportError } from '@lingara/api'
import { tokenHandoff, type EmbedServer, type MintedToken } from '@lingara/embed/server'

import type { Json } from './receiver.js'
import type { Recording } from './recording.js'

/** What the jobs left behind. */
export interface Outcome {
  minted: MintedToken | null
  handoff: string | null
  error: Json | null
  render: boolean
}

/** Every job in order; a raised error is recorded and the next job runs. */
export async function run(server: EmbedServer, jobs: Json[]): Promise<Outcome> {
  const out: Outcome = { minted: null, handoff: null, error: null, render: false }
  for (const job of jobs) {
    try {
      await one(server, job, out)
    } catch (error) {
      out.error = raised(error)
    }
  }
  return out
}

async function one(server: EmbedServer, job: Json, out: Outcome): Promise<void> {
  const mint = job['mint'] as { player_ref: string; scopes?: string[]; origin?: string } | undefined
  const del = job['delete'] as { player_ref: string } | undefined
  if (mint !== undefined) {
    out.minted = await server.mintToken(mint.player_ref, { scopes: mint.scopes, origin: mint.origin })
  } else if (del !== undefined) {
    await server.deletePlayer(del.player_ref)
  } else if ('handoff' in job) {
    out.handoff = out.minted === null ? null : tokenHandoff(out.minted)
  } else {
    out.render = true
  }
}

/** `{status, code}`: an API error's, an OAuth error's `error`, `maintenance`, a transport error's kind. */
function raised(error: unknown): Json {
  if (error instanceof ApiError) return { status: error.status, code: error.code }
  if (error instanceof OAuthError) return { status: error.status, code: error.error }
  if (error instanceof MaintenanceError) return { status: 503, code: 'maintenance' }
  if (error instanceof TransportError) return { status: 0, code: error.kind }
  return { status: 0, code: String(error) }
}

/** `value` with every null object member dropped, at any depth, and keys sorted: two values are JSON-equal when these strings are. */
export function canonical(value: unknown): string {
  const strip = (v: unknown): unknown => {
    if (Array.isArray(v)) return v.map(strip)
    if (v === null || typeof v !== 'object') return v
    const entries = Object.entries(v as Json).filter(([, item]) => item !== null && item !== undefined)
    return Object.fromEntries(entries.sort(([a], [b]) => (a < b ? -1 : 1)).map(([k, item]) => [k, strip(item)]))
  }
  return JSON.stringify(strip(value)) ?? 'nothing'
}

/** The six fields in wire form, the token read through its accessor. */
function wire(t: MintedToken): Json {
  return {
    token: t.exposeToken(),
    expires_at: t.expiresAt,
    expires_in: t.expiresIn,
    subject: t.subject,
    scopes: t.scopes,
    account_linked: t.accountLinked,
  }
}

const KEYS = ['minted', 'handoff', 'error', 'directory', 'directory_lookups'] as const

/** `expect.minted`, `.handoff`, `.error`, `.directory` and `.directory_lookups`, each a `fail <key>` line when it differs. */
export function judge(id: string, expect: Json, out: Outcome, directory: Recording): string[] {
  const got: Record<(typeof KEYS)[number], unknown> = {
    minted: out.minted === null ? null : wire(out.minted),
    handoff: out.handoff,
    error: out.error,
    directory: directory.snapshot(),
    directory_lookups: directory.lookups,
  }
  const found: string[] = []
  for (const key of KEYS) {
    const want = expect[key]
    const have = got[key]
    // A key the case names must match; an error the case did not expect is a mismatch too.
    const differs = want === undefined ? key === 'error' && have !== null : canonical(want) !== canonical(have)
    if (differs) found.push(`${id}: fail ${key}: got ${canonical(have)}, expected ${canonical(want)}`)
  }
  return found
}
