import { readFileSync } from 'node:fs'

import { defineConfig } from 'vitest/config'

// The server half's version, as scripts/build-server.mjs defines it for dist/.
const { version } = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8'))

// Shared by both halves of the package. The default environment is `node`, so
// the server tests and the browser's SSR test need nothing; a browser test
// that needs a DOM opens with `/** @vitest-environment jsdom */`.
export default defineConfig({
  define: { __LINGARA_EMBED_VERSION__: JSON.stringify(version) },
  test: {
    environment: 'node',
    include: ['src/**/__tests__/**/*.test.ts'],
  },
})
