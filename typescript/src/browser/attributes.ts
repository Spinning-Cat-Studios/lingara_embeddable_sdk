// The three attributes checked before a frame is built, the origin's
// normalisation, and the frame URL. Nothing here touches the DOM.

import { SURFACES, THEMES, type Surface, type Theme } from './protocol.js'

export const DEFAULT_ORIGIN = 'https://app.getlingara.com'

// The Backend reads plan ids as UUIDs; the check is form only, so any version
// and the nil UUID pass.
const PLAN_ID = /^[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}$/
const LOOPBACK = ['localhost', '127.0.0.1']

export type AttributeCode = 'surface_unsupported' | 'plan_not_found' | 'lingara_origin_invalid'

/** Where one session's frame points. */
export interface FrameTarget {
  surface: Surface
  planId: string
  origin: string
}

export type Checked = { ok: true; target: FrameTarget } | { ok: false; code: AttributeCode }

/**
 * `https://` plus a host, or `http://localhost` / `http://127.0.0.1` with an
 * optional port; a trailing `/` is allowed, any other path, query, fragment
 * or userinfo is not. Returns `new URL(value).origin`, the form the browser
 * gives `MessageEvent.origin`, so the two compare with `===`.
 */
export function normaliseOrigin(value: string): string | null {
  // `URL` drops an empty `?` or `#`, so the raw string is checked first.
  if (/[?#]/.test(value)) return null
  let url: URL
  try {
    url = new URL(value)
  } catch {
    return null
  }
  const secure = url.protocol === 'https:' && url.hostname !== ''
  const loopback = url.protocol === 'http:' && LOOPBACK.includes(url.hostname)
  if (!(secure || loopback) || url.username !== '' || url.password !== '' || url.pathname !== '/') return null
  return url.origin
}

/** Absent or empty is `practice`; anything outside the closed set is `null`. */
export function resolveSurface(value: string | null): Surface | null {
  if (value === null || value === '') return 'practice'
  return SURFACES.find((surface) => surface === value) ?? null
}

export const isPlanId = (value: string | null): value is string => value !== null && PLAN_ID.test(value)

/** Any value outside the closed set is treated as absent. */
export function resolveTheme(value: string | null): Theme | undefined {
  return THEMES.find((theme) => theme === value)
}

/** An absent or empty `locale` leaves the choice to the page. */
export const resolveLocale = (value: string | null): string | undefined => (value ? value : undefined)

/** The checks a session runs before it builds a frame, in the order a host reads them. */
export function checkAttributes(read: (name: string) => string | null): Checked {
  const surface = resolveSurface(read('surface'))
  if (surface === null) return { ok: false, code: 'surface_unsupported' }
  const planId = read('plan-id')
  if (!isPlanId(planId)) return { ok: false, code: 'plan_not_found' }
  const raw = read('lingara-origin')
  const origin = raw === null ? DEFAULT_ORIGIN : normaliseOrigin(raw)
  if (origin === null) return { ok: false, code: 'lingara_origin_invalid' }
  return { ok: true, target: { surface, planId, origin } }
}

/** The frame URL: never the token, never the plan id. */
export function frameSrc(target: FrameTarget, locale: string | undefined, theme: Theme | undefined): string {
  let src = `${target.origin}/embed/${target.surface}?v=1`
  if (locale !== undefined) src += `&lng=${encodeURIComponent(locale)}`
  if (theme !== undefined) src += `&theme=${encodeURIComponent(theme)}`
  return src
}
