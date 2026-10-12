// The server half's place in the package (ADR 1.10.26af D1, D2): the
// library is an optional peer, never a dependency, so a site that only
// mounts the widget installs nothing; the browser entry never reaches
// `./server`; and the server entry without the library fails at import,
// naming it.

import { spawnSync } from 'node:child_process'
import { copyFileSync, mkdtempSync, readdirSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

import { describe, expect, it } from 'vitest'

const root = new URL('../../../', import.meta.url)
const read = (path: string): string => readFileSync(new URL(path, root), 'utf8')

const IMPORT = /(?:\bfrom\s*|\bimport\s*\(?\s*|\brequire\s*\(\s*)['"]([^'"]+)['"]/g

describe('1.10.26af — the package', () => {
  /** 1.10.26af AC28: `@lingara/api` is an optional peer and not a runtime dependency, and the browser entry imports nothing from `./server`. */
  it('the library is an optional peer of the server subpath', async () => {
    const pkg = JSON.parse(read('package.json'))
    const floor = read('../LIBRARY_FLOOR').trim()
    expect(pkg.dependencies).toEqual({})
    expect(pkg.peerDependencies).toEqual({ '@lingara/api': `^${floor}` })
    expect(pkg.peerDependenciesMeta).toEqual({ '@lingara/api': { optional: true } })
    expect(pkg.devDependencies['@lingara/api']).toBe(floor)
    expect(pkg.exports['./server']).toEqual({
      import: { types: './dist/server/index.d.ts', default: './dist/server/index.js' },
      require: { types: './dist/server/index.d.cts', default: './dist/server/index.cjs' },
    })
    for (const condition of Object.values(pkg.exports['./server']) as Record<string, string>[]) {
      for (const path of Object.values(condition)) expect(() => read(path), path).not.toThrow()
    }

    // The browser half: no specifier reaches the server half or the library.
    const browser = new URL('src/browser/', root)
    for (const name of readdirSync(browser).filter((file) => file.endsWith('.ts'))) {
      for (const [, specifier] of readFileSync(new URL(name, browser), 'utf8').matchAll(IMPORT)) {
        expect(specifier, `${name} imports ${specifier}`).not.toMatch(/server|@lingara\/api/)
      }
    }
    for (const built of ['dist/browser/index.js', 'dist/browser/define.js', 'dist/lingara-embed.min.js']) {
      expect(read(built), built).not.toMatch(/@lingara\/api|\/server/)
    }

    // The server entry, alone in a directory with no node_modules above it.
    const lonely = mkdtempSync(join(tmpdir(), 'lingara-embed-server-'))
    try {
      // `.mjs`: outside the package, no `"type": "module"` says what a `.js` is.
      copyFileSync(new URL('dist/server/index.js', root), join(lonely, 'index.mjs'))
      copyFileSync(new URL('dist/server/index.cjs', root), join(lonely, 'index.cjs'))
      // A child node, so the test runner's own resolver cannot find the library.
      for (const [flag, code] of [
        ['--input-type=module', `await import(${JSON.stringify(pathToFileURL(join(lonely, 'index.mjs')).href)})`],
        ['--input-type=commonjs', `require(${JSON.stringify(join(lonely, 'index.cjs'))})`],
      ]) {
        const run = spawnSync(process.execPath, [flag, '-e', code], { cwd: lonely, encoding: 'utf8' })
        expect(run.status, code).not.toBe(0)
        expect(run.stderr, code).toMatch(/Cannot find (package|module) '@lingara\/api'/)
      }
    } finally {
      rmSync(lonely, { recursive: true, force: true })
    }
  })
})
