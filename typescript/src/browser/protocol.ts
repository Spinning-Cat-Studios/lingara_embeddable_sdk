// The host half of the postMessage protocol, v1: the envelope, the closed
// sets, and the two pure functions `__tests__/protocol-v1.json` holds to the
// frame's own copy. An additive change (a type, an optional field, an error
// code, a surface) stays v1; anything else is v2.

export const PROTOCOL = 'lingara-embed'
export const VERSION = 1

export const SURFACES = ['practice'] as const
export type Surface = (typeof SURFACES)[number]

export const ERROR_CODES = [
  'origin_refused',
  'token_invalid',
  'token_expired',
  'scope_missing',
  'plan_not_found',
  'plan_not_ready',
  'surface_unsupported',
  'version_unsupported',
  'embed_unavailable',
  'rate_limited',
  'network',
  'internal',
] as const
export type ErrorCode = (typeof ERROR_CODES)[number]

export const THEMES = ['light', 'dark', 'auto'] as const
export type Theme = (typeof THEMES)[number]

export const HOST_TYPES = ['init', 'token', 'configure'] as const
export const FRAME_TYPES = ['ready', 'started', 'token_request', 'resized', 'practice_completed', 'error'] as const

/** What the host sends, by type. */
export interface HostPayloads {
  init: { token: string; plan_id: string }
  token: { token: string }
  configure: { theme?: Theme; locale?: string }
}
export type HostType = keyof HostPayloads

export interface PracticeCompleted {
  plan_id: string
  set_number: number
  correct: number
  total: number
  event_id: string | null
}

/** What the frame sends, by type, narrowed to the fields the host reads. */
export interface FramePayloads {
  ready: { versions: number[]; surfaces: string[] }
  started: { surface: string; plan_id: string }
  token_request: { reason?: string }
  resized: { height: number }
  practice_completed: PracticeCompleted
  error: { code: string; retryable: boolean }
}
export type FrameType = keyof FramePayloads
export type FrameMessage = { [T in FrameType]: { type: T; payload: FramePayloads[T] } }[FrameType]

export interface Envelope<T extends string, P> {
  protocol: typeof PROTOCOL
  version: typeof VERSION
  type: T
  payload: P
}

type Fields = Record<string, unknown>

function isObject(value: unknown): value is Fields {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

const isString = (value: unknown): value is string => typeof value === 'string'
const isNumber = (value: unknown): value is number => typeof value === 'number' && Number.isFinite(value)
const isList = <T>(value: unknown, item: (v: unknown) => v is T): value is T[] =>
  Array.isArray(value) && value.every(item)

function readCompleted(p: Fields): PracticeCompleted | null {
  const { plan_id, set_number, correct, total } = p
  if (!isString(plan_id) || !isNumber(set_number) || !isNumber(correct) || !isNumber(total)) return null
  return { plan_id, set_number, correct, total, event_id: isString(p.event_id) ? p.event_id : null }
}

// Each reader copies only its own fields, so a newer frame's extra field never
// travels further than this file.
const READERS: { [T in FrameType]: (p: Fields) => FramePayloads[T] | null } = {
  ready: (p) =>
    isList(p.versions, isNumber) && isList(p.surfaces, isString)
      ? { versions: [...p.versions], surfaces: [...p.surfaces] }
      : null,
  started: (p) => (isString(p.surface) && isString(p.plan_id) ? { surface: p.surface, plan_id: p.plan_id } : null),
  token_request: (p) => (isString(p.reason) ? { reason: p.reason } : {}),
  resized: (p) => (isNumber(p.height) ? { height: p.height } : null),
  practice_completed: readCompleted,
  error: (p) => (isString(p.code) ? { code: p.code, retryable: p.retryable === true } : null),
}

/**
 * One frame message, or `null` for anything the host ignores: a non-object,
 * another `protocol`, a non-object `payload`, an unknown `type`, a missing
 * field. `ready` is read at any envelope version, because its `versions` is
 * the negotiation; every other type only at v1.
 */
export function parseFrameMessage(data: unknown): FrameMessage | null {
  if (!isObject(data) || data.protocol !== PROTOCOL || !isObject(data.payload)) return null
  const type = FRAME_TYPES.find((known) => known === data.type)
  if (type === undefined) return null
  if (type === 'ready' ? !isNumber(data.version) : data.version !== VERSION) return null
  const payload = READERS[type](data.payload)
  return payload === null ? null : ({ type, payload } as FrameMessage)
}

/** One host message, in the envelope. */
export function hostMessage<T extends HostType>(type: T, payload: HostPayloads[T]): Envelope<T, HostPayloads[T]> {
  return { protocol: PROTOCOL, version: VERSION, type, payload }
}
