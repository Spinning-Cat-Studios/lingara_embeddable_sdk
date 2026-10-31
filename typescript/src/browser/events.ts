// The DOM events the element dispatches, their `detail` builders and the
// element's own error codes. Every `detail` is a fresh object built field by
// field, so a newer frame's extra payload field never reaches the host and no
// event ever carries a token.

import { ERROR_CODES, type FramePayloads } from './protocol.js'

export const EVENTS = {
  ready: 'lingara:ready',
  resize: 'lingara:resize',
  practiceCompleted: 'lingara:practice-completed',
  error: 'lingara:error',
  updateAvailable: 'lingara:update-available',
} as const

/** The element's own codes and their `retryable`; three of them are the frame's codes reused. */
export const ELEMENT_CODES = {
  surface_unsupported: false,
  plan_not_found: false,
  version_unsupported: false,
  lingara_origin_invalid: false,
  token_provider_invalid: true,
  token_provider_failed: true,
} as const
export type ElementCode = keyof typeof ELEMENT_CODES

export const MIN_HEIGHT = 120
export const MAX_HEIGHT = 4000

export interface ReadyDetail {
  surface: string
  planId: string
}

export interface ResizeDetail {
  height: number
}

export interface PracticeCompletedDetail {
  planId: string
  setNumber: number
  correct: number
  total: number
  /** The id of the row that recorded the result, or `null` if recording failed; not the webhook's id. */
  eventId: string | null
}

export interface ErrorDetail {
  code: string
  retryable: boolean
}

/**
 * The handoff's `update_available` (ADR 30.10.26p): the version now serving
 * (`null` when none is published), whether yours is deprecated, and the
 * developer's message. For your console, never the player's screen.
 */
export interface UpdateAvailableDetail {
  latest: string | null
  deprecated: boolean
  message: string | null
}

/** V1's schema caps `message` at 280; the line cuts again, in code points. */
const MESSAGE_CAP = 280

export const updateDetail = (u: UpdateAvailableDetail): UpdateAvailableDetail => ({
  latest: u.latest,
  deprecated: u.deprecated,
  message: u.message,
})

/**
 * The core's log line for the notice, as `conformance/vectors/update-notice.json`
 * pins it. The widget has no `embed_version`, so the subject is always this
 * build's. It never compares versions: `latest` can be below a yanked build.
 */
export function updateLine(u: UpdateAvailableDetail): string {
  const state = u.deprecated ? 'is deprecated' : 'is not the current version'
  const current = u.latest === null ? 'no version is currently published' : `the current version is ${u.latest}`
  const line = `lingara: this build's embed version ${state}; ${current}`
  return u.message === null ? line : `${line}: ${Array.from(u.message).slice(0, MESSAGE_CAP).join('')}`
}

export const clampHeight =(height: number): number => Math.min(MAX_HEIGHT, Math.max(MIN_HEIGHT, height))

export const readyDetail = (p: FramePayloads['started']): ReadyDetail => ({ surface: p.surface, planId: p.plan_id })

export const resizeDetail = (height: number): ResizeDetail => ({ height })

export const completedDetail = (p: FramePayloads['practice_completed']): PracticeCompletedDetail => ({
  planId: p.plan_id,
  setNumber: p.set_number,
  correct: p.correct,
  total: p.total,
  eventId: p.event_id,
})

/** A frame code outside the frame's twelve, an element-only spelling included, becomes `internal`. */
export function frameErrorDetail(p: FramePayloads['error']): ErrorDetail {
  const known = (ERROR_CODES as readonly string[]).includes(p.code)
  return known ? { code: p.code, retryable: p.retryable } : { code: 'internal', retryable: false }
}

export const elementErrorDetail = (code: ElementCode): ErrorDetail => ({ code, retryable: ELEMENT_CODES[code] })

export function emit(target: EventTarget, name: string, detail: object): void {
  target.dispatchEvent(new CustomEvent(name, { detail, bubbles: true, composed: true }))
}
