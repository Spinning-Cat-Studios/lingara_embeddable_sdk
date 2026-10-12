// The widget snippet (ADR 1.10.26af D7, ES4): the `<lingara-embed>` element
// and its two module scripts, rendered identically by all eight server kits.
// The markup is the widget's own (K6); the token never appears in it, only
// the endpoint the element asks at run time.

// Replaced at build time (scripts/build-server.mjs, vitest.config.ts) with
// package.json's version: one VERSION for the whole trio, never typed here.
declare const __LINGARA_EMBED_VERSION__: string

/** This package's version: the element the default `elementSrc` loads. */
export const VERSION: string = __LINGARA_EMBED_VERSION__

/** The script-tag build of this release's element. */
export const DEFAULT_ELEMENT_SRC = `https://cdn.jsdelivr.net/npm/@lingara/embed@${VERSION}/dist/lingara-embed.min.js`

/** What to render. `planId` and `tokenEndpoint` are required; every other option is written only when set. */
export interface SnippetOptions {
  /** The lesson plan your server made for this player, a UUID. */
  planId: string
  /** Your token route: a path (`/lingara/token`) or an `https://` URL. The element `POST`s to it with the page's cookies and reads `tokenHandoff`'s body. */
  tokenEndpoint: string
  /** Default `practice`. */
  surface?: string | undefined
  theme?: 'light' | 'dark' | 'auto' | undefined
  locale?: string | undefined
  lingaraOrigin?: string | undefined
  title?: string | undefined
  /** Default `lingara-embed`; two widgets on one page need two ids. */
  id?: string | undefined
  /** Your page's CSP nonce, written on both scripts. */
  nonce?: string | undefined
  /** Default {@link DEFAULT_ELEMENT_SRC}; an override must be `https://`. */
  elementSrc?: string | undefined
}

/** The first option refused, by its contract name (`planId`, `surface`, …). It never carries the value. */
export class SnippetError extends Error {
  readonly field: string

  constructor(field: string) {
    super(`the widget snippet's ${field} is not valid`)
    this.name = 'SnippetError'
    this.field = field
  }
}

/** Unicode category Cc. */
const CONTROL = /[\u0000-\u001f\u007f-\u009f]/u // eslint-disable-line no-control-regex
/** The contract's whitespace set (Unicode `White_Space` beyond Cc). */
const SPACE = /[ \u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]/u

const PLAN_ID = /^[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}$/
const SURFACE = /^[a-z][a-z0-9-]{0,31}$/
const LOCALE = /^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}$/
const ORIGIN = /^(https:\/\/[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*|http:\/\/(localhost|127\.0\.0\.1))(:[0-9]{1,5})?\/?$/
const ID = /^[A-Za-z][A-Za-z0-9_-]{0,63}$/
const NONCE = /^[A-Za-z0-9+/=_-]{8,128}$/

/** At most 2048 bytes, with no control character or whitespace. */
const urlText = (s: string): boolean => new TextEncoder().encode(s).length <= 2048 && !CONTROL.test(s) && !SPACE.test(s)

type Rule = (value: string) => boolean

const RULES: ReadonlyArray<readonly [keyof SnippetOptions, Rule]> = [
  ['planId', (s) => PLAN_ID.test(s)],
  ['surface', (s) => SURFACE.test(s)],
  ['theme', (s) => s === 'light' || s === 'dark' || s === 'auto'],
  ['locale', (s) => LOCALE.test(s)],
  ['lingaraOrigin', (s) => ORIGIN.test(s)],
  ['title', (s) => [...s].length >= 1 && [...s].length <= 80 && !CONTROL.test(s)],
  ['id', (s) => ID.test(s)],
  ['tokenEndpoint', (s) => ((s.startsWith('/') && !s.startsWith('//')) || s.startsWith('https://')) && urlText(s)],
  ['nonce', (s) => NONCE.test(s)],
  ['elementSrc', (s) => s.startsWith('https://') && urlText(s)],
]

const ATTRIBUTE_ESCAPES: Record<string, string> = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }
const SCRIPT_ESCAPES: Record<string, string> = {
  '"': '\\"',
  '\\': '\\\\',
  '<': '\\u003c',
  '>': '\\u003e',
  '&': '\\u0026',
  '\u2028': '\\u2028',
  '\u2029': '\\u2029',
}

/** An attribute value: the five characters, and nothing else. */
const attr = (value: string): string => value.replace(/[&<>"']/g, (c) => ATTRIBUTE_ESCAPES[c] ?? c)

/** A JSON string literal that can never close the script or open a comment. */
const scriptString = (value: string): string => `"${value.replace(/["\\<>&\u2028\u2029]/g, (c) => SCRIPT_ESCAPES[c] ?? c)}"`

type Resolved = SnippetOptions & { surface: string; id: string; elementSrc: string }

/** The defaults filled in. The two required options are checked even when a JavaScript caller omits them. */
function resolve(options: SnippetOptions): Resolved {
  return {
    ...options,
    planId: options.planId ?? '',
    tokenEndpoint: options.tokenEndpoint ?? '',
    surface: options.surface ?? 'practice',
    id: options.id ?? 'lingara-embed',
    elementSrc: options.elementSrc ?? DEFAULT_ELEMENT_SRC,
  }
}

/** Each rule in the contract's order; the first failure names its option. */
function check(o: Resolved): void {
  for (const [field, ok] of RULES) {
    const value = o[field]
    if (value !== undefined && !(typeof value === 'string' && ok(value))) throw new SnippetError(field)
  }
}

/** The three lines to paste into your page, with no trailing newline. Throws {@link SnippetError} on the first invalid option. */
export function widgetSnippet(options: SnippetOptions): string {
  const o = resolve(options)
  check(o)
  let element = `<lingara-embed id="${attr(o.id)}" surface="${attr(o.surface)}" plan-id="${attr(o.planId)}"`
  const optional = [['theme', o.theme], ['locale', o.locale], ['lingara-origin', o.lingaraOrigin], ['title', o.title]] as const
  for (const [name, value] of optional) {
    if (value !== undefined) element += ` ${name}="${attr(value)}"`
  }
  const nonce = o.nonce === undefined ? '' : ` nonce="${attr(o.nonce)}"`
  const provider = `document.getElementById(${scriptString(o.id)}).tokenProvider = async () => (await fetch(${scriptString(o.tokenEndpoint)}, { method: "POST", credentials: "same-origin" })).json();`
  return [
    `${element}></lingara-embed>`,
    `<script type="module"${nonce} src="${attr(o.elementSrc)}"></script>`,
    `<script type="module"${nonce}>${provider}</script>`,
  ].join('\n')
}
