// The handoff body (ADR 1.10.26af D6, ES6): exactly
// `{"token":"lgr_et_…","expires_at":"…"}`, the shape every player kit and
// the widget read. This is the one place a kit exposes the token on purpose.

import type { MintedToken } from '@lingara/api'

/**
 * The JSON your token route returns to the player. `expires_at` is the
 * server's string, copied byte for byte; `expires_in` and `subject` are not
 * forwarded.
 */
export function tokenHandoff(minted: MintedToken): string {
  return handoffBody(minted.exposeToken(), minted.expiresAt)
}

/**
 * The pure body over two strings, held to `token-handoff.json`: compact,
 * these two keys in this order, `/` and non-ASCII unescaped. Internal: not
 * exported from the package.
 */
export function handoffBody(token: string, expiresAt: string): string {
  return JSON.stringify({ token, expires_at: expiresAt })
}
