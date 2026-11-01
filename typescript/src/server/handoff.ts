// The handoff body (ADR 1.10.26af D6, ES6): exactly
// `{"token":"lgr_et_…","expires_at":"…"}`, the shape every player kit and
// the widget read, and since ADR 31.10.26f a third key, `update_available`,
// when the mint answered one. This is the one place a kit exposes the token
// on purpose.

import type { MintedToken, UpdateAvailable } from '@lingara/api'

/**
 * The JSON your token route returns to the player. `expires_at` is the
 * server's string, copied byte for byte; `expires_in` and `subject` are not
 * forwarded. The mint's `update_available` is forwarded when present.
 */
export function tokenHandoff(minted: MintedToken): string {
  return handoffBody(minted.exposeToken(), minted.expiresAt, minted.updateAvailable ?? undefined)
}

/**
 * The pure body, held to `token-handoff.json`: compact, `token` then
 * `expires_at`, then `update_available` only when given (its keys `latest`,
 * `deprecated`, `message`, nulls kept); `/` and non-ASCII unescaped.
 * Internal: not exported from the package.
 */
export function handoffBody(token: string, expiresAt: string, updateAvailable?: UpdateAvailable): string {
  if (updateAvailable === undefined) return JSON.stringify({ token, expires_at: expiresAt })
  // Built from the typed value in the wire order (V1 §6), never a forwarded map.
  const notice = { latest: updateAvailable.latest, deprecated: updateAvailable.deprecated, message: updateAvailable.message }
  return JSON.stringify({ token, expires_at: expiresAt, update_available: notice })
}
