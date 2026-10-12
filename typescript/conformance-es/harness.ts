// The ES harness on the TypeScript server kit (ADR 1.10.26af D13): every ES
// case through `@lingara/embed/server` and nothing else, run from the
// package's built `dist/server` (the package resolves its own name).
//
// `conformance-server run --tier es --lang typescript -- <this>` starts it,
// with the repository root as its cwd, the server's URL, the results path,
// the receiver's two webhook secrets and the visibly fake client
// credentials. It binds its webhook receiver to port 0, prints
// `listening <port>`, and then for each case: fetches it, builds a fresh
// client, server and directory, arms it, runs the `kit:` jobs, waits for
// every delivery, judges what only the caller sees, finishes, and writes one
// result line. There is no skip.

import { appendFileSync, readdirSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:http'
import type { AddressInfo } from 'node:net'
import { join } from 'node:path'

import { env, runCase } from './cases.js'
import { Receiver } from './receiver.js'
import { captureConsole, Sink } from './recording.js'

/** `<group>/<slug>.yaml` → `<group>.<slug>`, sorted. */
function caseIds(dir: string): string[] {
  const ids: string[] = []
  for (const group of readdirSync(dir, { withFileTypes: true }).filter((entry) => entry.isDirectory())) {
    for (const file of readdirSync(join(dir, group.name)).filter((name) => name.endsWith('.yaml'))) {
      ids.push(`${group.name}.${file.slice(0, -'.yaml'.length)}`)
    }
  }
  return ids.sort()
}

async function main(): Promise<void> {
  const sink = new Sink()
  const [primary = '', secondary = ''] = env('LINGARA_EMBED_WEBHOOK_SECRETS').split(',')
  const receiver = new Receiver(new Map([['primary', primary], ['secondary', secondary]]), sink)
  const http = createServer(receiver.listener)
  await new Promise<void>((resolve) => http.listen(0, '127.0.0.1', resolve))
  process.stdout.write(`listening ${(http.address() as AddressInfo).port}\n`)
  captureConsole(sink)

  const results = env('LINGARA_EMBED_RESULTS')
  writeFileSync(results, '')
  for (const id of caseIds(process.env['LINGARA_EMBED_CASES'] ?? 'conformance/es/cases')) {
    appendFileSync(results, `${JSON.stringify(await runCase(receiver, id))}\n`)
  }
  // The conformance server may hold keep-alive connections to the receiver.
  http.closeAllConnections()
  http.close()
}

await main()
