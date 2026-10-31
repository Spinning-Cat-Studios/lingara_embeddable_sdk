// The token's one clock: grant validation, the `expires_at` parser, the
// re-mint schedule, single-flight and back-off. The token goes to the caller's
// `granted` hook and nowhere else; it is never stored, logged or thrown.

import type { UpdateAvailableDetail } from './events.js'

/**
 * The integrator's token endpoint answer. `update_available` is the mint's
 * optional notice (ADR 30.10.26p), forwarded as the server answered it; a
 * malformed one is dropped and the grant still works. Other fields are ignored.
 */
export interface TokenGrant {
  token: string
  expires_at: string
  update_available?: UpdateAvailableDetail | null
}
export type TokenProvider = () => Promise<TokenGrant>

export interface ParsedGrant {
  token: string
  expiresAt: number
  /** Present only when the grant carried a well-formed notice. */
  update?: UpdateAvailableDetail
}

const textOrNull = (value: unknown): value is string | null => value === null || value === undefined || typeof value === 'string'

/** V1's three keys, or `undefined` for anything else: a notice never costs the grant. */
export function readUpdate(value: unknown): UpdateAvailableDetail | undefined {
  if (typeof value !== 'object' || value === null) return undefined
  const { latest, deprecated, message } = value as Record<string, unknown>
  if (typeof deprecated !== 'boolean' || !textOrNull(latest) || !textOrNull(message)) return undefined
  return { latest: latest ?? null, deprecated, message: message ?? null }
}

export const LEAD_MS = 60_000
export const MIN_DELAY_MS = 30_000
// The token lives 900 s and nothing longer is minted, so 900 s less the lead.
// It also keeps every delay far below `setTimeout`'s 2³¹−1 ms limit.
export const MAX_DELAY_MS = 840_000
/** The waits between attempts; the failure after the last one is reported. */
export const BACKOFF_MS = [2_000, 8_000, 30_000]

const RFC3339 = /^(\d{4})-(\d{2})-(\d{2})[Tt](\d{2}):(\d{2}):(\d{2})(?:\.(\d{1,9}))?(?:[Zz]|([+-])(\d{2}):(\d{2}))$/
const DAYS = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]

const leap = (year: number): boolean => (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0
const daysIn = (year: number, month: number): number => (month === 2 && leap(year) ? 29 : DAYS[month - 1])
const within = (value: number, low: number, high: number): boolean => value >= low && value <= high

// `Date.UTC` rolls `2026-13-45` over to a real date, so every field is checked
// first. A leap second (`:60`) is refused: the server never writes one.
function inRange([year, month, day, hour, minute, second, offsetHour, offsetMinute]: number[]): boolean {
  return (
    within(month, 1, 12) &&
    within(day, 1, daysIn(year, month)) &&
    hour <= 23 &&
    minute <= 59 &&
    second <= 59 &&
    offsetHour <= 23 &&
    offsetMinute <= 59
  )
}

/**
 * RFC 3339 to epoch milliseconds, or `null`. Not `Date.parse`, which the
 * language pins only for exactly three fraction digits: a server may write up
 * to nine, truncated here to milliseconds.
 */
export function parseExpiresAt(value: string): number | null {
  const match = RFC3339.exec(value)
  if (match === null) return null
  const [, year, month, day, hour, minute, second, fraction = '', sign = '+', offH = '0', offM = '0'] = match
  const fields = [year, month, day, hour, minute, second, offH, offM].map(Number)
  if (!inRange(fields)) return null
  const millis = Number(`${fraction}000`.slice(0, 3))
  const offset = (sign === '-' ? -1 : 1) * (fields[6] * 60 + fields[7]) * 60_000
  return Date.UTC(fields[0], fields[1] - 1, fields[2], fields[3], fields[4], fields[5], millis) - offset
}

/** The two fields, checked; `null` for anything else. A past `expires_at` is not a refusal. */
export function validateGrant(answer: unknown): ParsedGrant | null {
  if (typeof answer !== 'object' || answer === null) return null
  const { token, expires_at: expiresAtText, update_available: notice } = answer as Record<string, unknown>
  if (typeof token !== 'string' || !token.startsWith('lgr_et_') || typeof expiresAtText !== 'string') return null
  const expiresAt = parseExpiresAt(expiresAtText)
  if (expiresAt === null) return null
  const update = readUpdate(notice)
  return update === undefined ? { token, expiresAt } : { token, expiresAt, update }
}

/** `clamp(expires_at − 60 s − now, 30 s, 840 s)`. */
export const remintDelay = (expiresAt: number, now: number): number =>
  Math.min(MAX_DELAY_MS, Math.max(MIN_DELAY_MS, expiresAt - LEAD_MS - now))

type Outcome = ParsedGrant | 'invalid' | 'failed'

async function ask(provider: TokenProvider): Promise<Outcome> {
  try {
    return validateGrant(await provider()) ?? 'invalid'
  } catch {
    return 'failed'
  }
}

export interface KeeperHooks {
  provider(): TokenProvider | undefined
  granted(token: string, update?: UpdateAvailableDetail): void
  failed(code: 'token_provider_invalid' | 'token_provider_failed'): void
}

/** One element's token: held across sessions, dropped on disconnect. */
export class TokenKeeper {
  private token: string | undefined
  private expiresAt = 0
  private inflight = false
  private timer: ReturnType<typeof setTimeout> | undefined
  private retrying = false
  private failures = 0
  private exhausted = false
  private invalidReported = false
  // Bumped by `halt`, so a call that settles afterwards is discarded.
  private epoch = 0

  constructor(private readonly hooks: KeeperHooks) {}

  /** The held token while it has more than 60 s left. */
  fresh(): string | undefined {
    return this.token !== undefined && this.expiresAt - Date.now() > LEAD_MS ? this.token : undefined
  }

  /** A provider call is in flight, or a back-off retry is scheduled. */
  busy(): boolean {
    return this.inflight || this.retrying
  }

  /** Call the provider now, sharing any call already in flight; cancels a pending retry. */
  request(): void {
    if (this.inflight) return
    const provider = this.hooks.provider()
    if (provider === undefined) return
    this.cancelTimer()
    this.inflight = true
    const epoch = this.epoch
    void ask(provider).then((outcome) => {
      if (epoch !== this.epoch) return
      this.inflight = false
      this.settle(outcome)
    })
  }

  /** A tab back in view re-mints a token with 60 s or less left. */
  visible(): void {
    if (this.token !== undefined && this.expiresAt - Date.now() <= LEAD_MS) this.request()
  }

  /** A new session: the back-off starts over and the schedule re-arms for a held token. */
  reset(): void {
    this.cancelTimer()
    this.failures = 0
    this.exhausted = false
    this.invalidReported = false
    if (this.token !== undefined) this.arm(remintDelay(this.expiresAt, Date.now()))
  }

  /** The session ended: no more calls, and an in-flight one is discarded. The token is kept for `reload()`. */
  halt(): void {
    this.cancelTimer()
    this.epoch += 1
    this.inflight = false
  }

  /** Disconnected: halt and drop the one reference. */
  drop(): void {
    this.halt()
    this.token = undefined
    this.expiresAt = 0
  }

  private settle(outcome: Outcome): void {
    if (typeof outcome === 'object') return this.succeed(outcome)
    if (outcome === 'invalid' && !this.invalidReported) {
      this.invalidReported = true
      this.hooks.failed('token_provider_invalid')
    }
    this.fail()
  }

  private succeed(grant: ParsedGrant): void {
    this.token = grant.token
    this.expiresAt = grant.expiresAt
    this.failures = 0
    // After the failure was reported, a success answers its one ask and no more.
    if (!this.exhausted) this.arm(remintDelay(grant.expiresAt, Date.now()))
    this.hooks.granted(grant.token, grant.update)
  }

  private fail(): void {
    if (this.exhausted) return
    this.failures += 1
    if (this.failures > BACKOFF_MS.length) {
      this.exhausted = true
      this.hooks.failed('token_provider_failed')
      return
    }
    this.arm(BACKOFF_MS[this.failures - 1])
    this.retrying = true
  }

  private arm(delay: number): void {
    this.cancelTimer()
    this.timer = setTimeout(() => {
      this.timer = undefined
      this.retrying = false
      this.request()
    }, delay)
  }

  private cancelTimer(): void {
    if (this.timer !== undefined) clearTimeout(this.timer)
    this.timer = undefined
    this.retrying = false
  }
}
