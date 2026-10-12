// `@lingara/embed`, the browser entry. Importing it defines nothing and touches
// no DOM global, so a server-rendered page may import it freely; the element
// class exists only once `defineLingaraEmbed()` runs in a browser.

import { createLingaraEmbedClass } from './element.js'

export type { LingaraEmbedElement } from './element.js'
export type { EmbedState } from './handshake.js'
export type { TokenGrant, TokenProvider } from './tokens.js'
export type { ErrorDetail, PracticeCompletedDetail, ReadyDetail, ResizeDetail, ElementCode } from './events.js'
export { EVENTS, ELEMENT_CODES } from './events.js'
export { ERROR_CODES } from './protocol.js'

/**
 * Registers `<lingara-embed>` (or `tagName`). Returns `true` when it
 * registers, and `false` with no effect where there are no custom elements
 * (a server, a worker) or the name is already taken, so two bundles on one
 * page never throw.
 */
export function defineLingaraEmbed(tagName = 'lingara-embed'): boolean {
  if (typeof customElements === 'undefined' || customElements.get(tagName) !== undefined) return false
  customElements.define(tagName, createLingaraEmbedClass())
  return true
}
