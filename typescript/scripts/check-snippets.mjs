// The web player snippets the docs page reads: each key exactly once, no
// region naming a secret, and every script — `widget.ts` and the inline
// module in `index.html` — bundled against the local, freshly built package.
//
// Usage: node typescript/scripts/check-snippets.mjs snippets/player/web
import { readdirSync, readFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { build } from 'esbuild'

const KEYS = ['mount', 'tokenProvider', 'events']
const MARKERS = { html: ['<!--', '-->'], ts: ['//', ''] }

const dir = resolve(process.argv[2] ?? 'snippets/player/web')
const entry = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist', 'browser', 'index.js')
const problems = []

/** Every `lingara:begin <key>` … `lingara:end` region in one file. */
function regions(file) {
  const [open, close] = MARKERS[file.split('.').pop()] ?? MARKERS.ts
  const found = []
  let current = null
  for (const line of readFileSync(join(dir, file), 'utf8').split('\n')) {
    const marker = line.trim().replace(close, '').trim()
    if (marker.startsWith(`${open} lingara:begin `)) {
      current = { key: marker.split(' ').pop(), file, body: [] }
    } else if (marker === `${open} lingara:end` && current !== null) {
      found.push(current)
      current = null
    } else if (current !== null) {
      current.body.push(line)
    }
  }
  if (current !== null) problems.push(`${file}: region ${current.key} never ends`)
  return found
}

const files = readdirSync(dir)
const all = files.filter((file) => /\.(ts|html)$/.test(file)).flatMap(regions)
for (const key of KEYS) {
  const homes = all.filter((region) => region.key === key).map((region) => region.file)
  if (homes.length !== 1) problems.push(`key ${key}: in ${homes.length} files (${homes.join(', ') || 'none'})`)
}
for (const region of all) {
  if (/secret|whsec_/i.test(region.body.join('\n'))) problems.push(`${region.file}: region ${region.key} names a secret`)
}
if (!readFileSync(join(dir, 'install.sh'), 'utf8').includes('{{version}}')) problems.push('install.sh: no {{version}}')

const local = { name: 'local', setup: (b) => b.onResolve({ filter: /^@lingara\/embed$/ }, () => ({ path: entry })) }
const bundle = (stdin) => build({ stdin: { ...stdin, resolveDir: dir }, bundle: true, write: false, logLevel: 'silent', plugins: [local] })
const html = readFileSync(join(dir, 'index.html'), 'utf8')
const inline = [...html.matchAll(/<script type="module">([\s\S]*?)<\/script>/g)].map((match) => match[1])
for (const [label, stdin] of [
  ['widget.ts', { contents: readFileSync(join(dir, 'widget.ts'), 'utf8'), loader: 'ts' }],
  ...inline.map((contents, n) => [`index.html script ${n + 1}`, { contents, loader: 'js' }]),
]) {
  await bundle(stdin).catch((error) => problems.push(`${label}: ${error.message.split('\n')[0]}`))
}
if (inline.length === 0) problems.push('index.html: no inline module script')

if (problems.length > 0) {
  for (const problem of problems) console.error(`✗ ${problem}`)
  process.exit(1)
}
console.log(`✓ ${dir}: ${KEYS.join(', ')} and install; ${inline.length + 1} script(s) build against the local package`)
