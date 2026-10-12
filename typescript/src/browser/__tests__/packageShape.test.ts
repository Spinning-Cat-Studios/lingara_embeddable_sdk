// The browser half has no dependency, a closed import graph and a size budget.

import { createHash } from 'node:crypto'
import { readdirSync, readFileSync } from 'node:fs'
import { gzipSync } from 'node:zlib'

import { build } from 'esbuild'
import { describe, expect, it } from 'vitest'

const root = new URL('../../../', import.meta.url)
const browser = new URL('src/browser/', root)
const read = (path: string): string => readFileSync(new URL(path, root), 'utf8')

/** 5 KiB gzip; raising it needs an ADR. */
const BUDGET_BYTES = 5 * 1024

const IMPORT = /(?:\bfrom\s*|\bimport\s*\(?\s*)['"]([^'"]+)['"]/g

describe('1.10.26ad — the package', () => {
  it('1.10.26ad AC9 the browser half has no dependency and fits its budget', async () => {
    const sources = readdirSync(browser).filter((name) => name.endsWith('.ts'))
    expect(sources.length).toBeGreaterThan(0)
    for (const name of sources) {
      const specifiers = [...readFileSync(new URL(name, browser), 'utf8').matchAll(IMPORT)].map((match) => match[1])
      for (const specifier of specifiers) {
        expect(specifier, `${name} imports ${specifier}`).toMatch(/^\.\/[a-z]+\.js$/i)
      }
    }

    const pkg = JSON.parse(read('package.json'))
    expect(pkg.dependencies).toEqual({})
    expect(pkg.files).toEqual(['dist/', 'README.md', 'LICENSE'])
    expect(pkg.exports['.']).toEqual({ types: './dist/browser/index.d.ts', import: './dist/browser/index.js' })
    expect(pkg.exports['./define']).toEqual({ types: './dist/browser/define.d.ts', import: './dist/browser/define.js' })
    for (const entry of [pkg.exports['.'], pkg.exports['./define']]) {
      for (const path of Object.values(entry) as string[]) expect(() => read(path), path).not.toThrow()
    }
    expect(read('LICENSE')).toBe(readFileSync(new URL('../LICENSE', root), 'utf8'))

    const bundle = await build({
      stdin: { contents: "export * from './index.ts'\nimport './define.ts'\n", resolveDir: browser.pathname, loader: 'ts' },
      bundle: true,
      minify: true,
      format: 'esm',
      target: 'es2020',
      write: false,
      // `sideEffects` names dist/, not the source, so the source `define` would be dropped.
      ignoreAnnotations: true,
      logLevel: 'silent',
    })
    expect(bundle.outputFiles[0].text).toContain('lingara-embed')
    const bundled = bundle.outputFiles[0].text
    const script = readFileSync(new URL('dist/lingara-embed.min.js', root))
    for (const [label, text] of [['entry + define', bundled], ['lingara-embed.min.js', script.toString('utf8')]]) {
      expect(gzipSync(text).length, label).toBeLessThanOrEqual(BUDGET_BYTES)
      expect(text, label).not.toContain('@lingara/api')
      expect(text, label).not.toContain('LingaraClient')
    }

    const integrity = `sha384-${createHash('sha384').update(script).digest('base64')}`
    expect(read('dist/lingara-embed.min.js.sri').trim()).toBe(integrity)
  })
})
