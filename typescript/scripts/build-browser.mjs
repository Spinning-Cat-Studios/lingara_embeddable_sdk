// The browser half's build (§6 of the element's design): the ESM entries and
// their declarations through tsc, then the self-contained script-tag module
// and its subresource-integrity value.
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { readFileSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { build } from 'esbuild'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const script = join(root, 'dist', 'lingara-embed.min.js')

rmSync(join(root, 'dist', 'browser'), { recursive: true, force: true })
execFileSync(join(root, 'node_modules', '.bin', 'tsc'), ['-p', join(root, 'tsconfig.browser.json')], { stdio: 'inherit' })

await build({
  entryPoints: [join(root, 'src', 'browser', 'define.ts')],
  outfile: script,
  bundle: true,
  minify: true,
  format: 'esm',
  target: 'es2020',
  platform: 'browser',
  legalComments: 'none',
})

const integrity = `sha384-${createHash('sha384').update(readFileSync(script)).digest('base64')}`
writeFileSync(`${script}.sri`, `${integrity}\n`)
console.log(`✓ dist/browser, dist/lingara-embed.min.js (${integrity})`)
