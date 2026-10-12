// The TypeScript server snippets the docs page reads (ADR 1.10.26af D13):
// P1's four region keys exactly once each, every region closed, no
// credential-shaped value anywhere, and the whole-file `install` carrying
// `{{version}}`. Their types are checked separately, by tsc against the
// built dist/server (make check-typescript).
//
// Usage: node typescript/scripts/check-server-snippets.mjs snippets/server/typescript
import { readdirSync, readFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

const KEYS = ['mint', 'deletePlayer', 'webhook', 'widgetSnippet']
const CREDENTIAL = /lgr_(cs|whsec|et|at)_[A-Za-z0-9+/=_-]{6,}/

const dir = resolve(process.argv[2] ?? 'snippets/server/typescript')
const problems = []

/** Every `// lingara:begin <key>` … `// lingara:end` in one file (29.9.26w D3's format, which P1 reads). */
function regions(file) {
  const found = []
  let current = null
  for (const line of readFileSync(join(dir, file), 'utf8').split('\n')) {
    const marker = line.trim()
    if (marker.startsWith('// lingara:begin ')) {
      if (current !== null) problems.push(`${file}: region ${current} opens inside another`)
      current = marker.slice('// lingara:begin '.length).trim()
    } else if (marker === '// lingara:end') {
      if (current === null) problems.push(`${file}: an endregion with no region`)
      else found.push({ key: current, file })
      current = null
    }
  }
  if (current !== null) problems.push(`${file}: region ${current} never ends`)
  return found
}

const files = readdirSync(dir)
const all = files.filter((file) => file.endsWith('.ts')).flatMap(regions)
for (const key of KEYS) {
  const homes = all.filter((region) => region.key === key).map((region) => region.file)
  if (homes.length !== 1) problems.push(`key ${key}: in ${homes.length} files (${homes.join(', ') || 'none'})`)
}
for (const region of all.filter((r) => !KEYS.includes(r.key))) problems.push(`${region.file}: unknown key ${region.key}`)
for (const file of files.filter((name) => /\.(ts|sh|json)$/.test(name))) {
  if (CREDENTIAL.test(readFileSync(join(dir, file), 'utf8'))) problems.push(`${file}: carries a credential-shaped value`)
}
if (!readFileSync(join(dir, 'install.sh'), 'utf8').includes('{{version}}')) problems.push('install.sh: no {{version}}')

if (problems.length > 0) {
  for (const problem of problems) console.error(`✗ ${problem}`)
  process.exit(1)
}
console.log(`✓ ${dir}: ${KEYS.join(', ')} and install`)
