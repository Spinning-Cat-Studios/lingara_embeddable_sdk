// The server half's build (ADR 1.10.26af D2): `dist/server/` as ESM
// (`index.js`, the package is `"type": "module"`) and CJS (`index.cjs`), as
// @lingara/api ships, each with its own declarations. tsc writes the `.d.ts`
// files (tsconfig.server.json); each is copied to a `.d.cts` whose relative
// imports name `.cjs`, so a `require` caller's types resolve to CJS too.
// esbuild writes the two JS files, `@lingara/api` left an import (a peer),
// and the version read from package.json, never typed by hand (D1).
import { execFileSync } from 'node:child_process'
import { readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { build } from 'esbuild'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const out = join(root, 'dist', 'server')
const { version } = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'))

rmSync(out, { recursive: true, force: true })
execFileSync(join(root, 'node_modules', '.bin', 'tsc'), ['-p', join(root, 'tsconfig.server.json')], { stdio: 'inherit' })
for (const name of readdirSync(out).filter((file) => file.endsWith('.d.ts'))) {
  const text = readFileSync(join(out, name), 'utf8').replace(/(from\s+['"]\.\/[^'"]+)\.js(['"])/g, '$1.cjs$2')
  writeFileSync(join(out, name.replace(/\.d\.ts$/, '.d.cts')), text)
}

for (const [format, file] of [['esm', 'index.js'], ['cjs', 'index.cjs']]) {
  await build({
    entryPoints: [join(root, 'src', 'server', 'index.ts')],
    outfile: join(out, file),
    bundle: true,
    format,
    platform: 'node',
    target: 'node20',
    external: ['@lingara/api'],
    define: { __LINGARA_EMBED_VERSION__: JSON.stringify(version) },
    legalComments: 'none',
  })
}
console.log(`✓ dist/server (ESM and CJS, @lingara/embed ${version})`)
