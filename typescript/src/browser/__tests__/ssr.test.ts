// Importing the browser entry on a server is inert: vitest's default `node`
// environment has no `HTMLElement`, `customElements` or `window`.

import { describe, expect, it } from 'vitest'

describe('1.10.26ad — SSR', () => {
  it('1.10.26ad AC1 importing the entry on a server is inert', async () => {
    expect(typeof HTMLElement).toBe('undefined')
    const before = Object.getOwnPropertyNames(globalThis).sort()

    const source = await import('../index.js')
    const built = await import('../../../dist/browser/index.js')

    expect(Object.getOwnPropertyNames(globalThis).sort()).toEqual(before)
    expect(source.defineLingaraEmbed()).toBe(false)
    expect(built.defineLingaraEmbed()).toBe(false)
    expect(Object.getOwnPropertyNames(globalThis).sort()).toEqual(before)
  })
})
