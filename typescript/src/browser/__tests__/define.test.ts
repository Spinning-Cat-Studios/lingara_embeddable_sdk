/** @vitest-environment jsdom */
// Defining is idempotent, and the script-tag build registers itself.

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { JSDOM } from 'jsdom'
import { describe, expect, it } from 'vitest'

import { defineLingaraEmbed } from '../index.js'

// Under jsdom `import.meta.url` is not a file URL, so the path is the module's directory.
const script = readFileSync(resolve(__dirname, '../../../dist/lingara-embed.min.js'), 'utf8')

describe('1.10.26ad — defining', () => {
  it('1.10.26ad AC12 defining is idempotent and the script build self-registers', () => {
    expect(defineLingaraEmbed()).toBe(true)
    expect(defineLingaraEmbed()).toBe(false)
    expect(customElements.get('lingara-embed')).toBeDefined()
    expect(defineLingaraEmbed('x-lingara')).toBe(true)
    expect(customElements.get('x-lingara')).toBeDefined()

    // The script build in a page of its own. It is an ES module, so its
    // top-level bindings are module-scoped: run it in a function scope, the
    // nearest a classic `eval` comes, and any global it writes still shows.
    const page = new JSDOM('<!doctype html><lingara-embed></lingara-embed>', { runScripts: 'outside-only' })
    const before = new Set(Object.getOwnPropertyNames(page.window))
    page.window.eval(`(function () {\n${script}\n})()`)
    expect(page.window.customElements.get('lingara-embed')).toBeDefined()
    expect(page.window.document.querySelector('lingara-embed')?.shadowRoot).not.toBeNull()
    expect(Object.getOwnPropertyNames(page.window).filter((key) => !before.has(key))).toEqual([])
  })
})
