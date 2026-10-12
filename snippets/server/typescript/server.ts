// The TypeScript server kit examples the documentation site shows (ADR
// 1.10.26af D13). Each marked region is vendored at a released tag; make
// check-typescript type-checks this file against the built package. Fetch API
// handlers, `(Request) => Promise<Response>`: the same lines sit inside Hono,
// a Next.js route handler, Bun or Deno, and Express through its raw body.

import { Lingara } from '@lingara/api'
import { EmbedServer, PlayerRouter, tokenHandoff, widgetSnippet, type PlayerDirectory } from '@lingara/embed/server'

// Your own session, database and queue.
declare function signedInUser(request: Request): Promise<{ id: string } | null>
declare const db: { query<T>(sql: string, params: unknown[]): Promise<T[]> }
declare const practiceJobs: { enqueue(playerRef: string, eventId: string): Promise<void> }

// lingara:begin webhook
// Your own table, beside your players:
//   CREATE TABLE lingara_players (player_ref TEXT NOT NULL, subject TEXT NOT NULL UNIQUE);
//   CREATE INDEX ON lingara_players (player_ref);
class PlayerTable implements PlayerDirectory {
  async remember(playerRef: string, subject: string): Promise<void> {
    await db.query('INSERT INTO lingara_players (player_ref, subject) VALUES ($1, $2) ON CONFLICT (subject) DO UPDATE SET player_ref = $1', [playerRef, subject])
  }
  async playerRefFor(subject: string): Promise<string | undefined> {
    const rows = await db.query<{ player_ref: string }>('SELECT player_ref FROM lingara_players WHERE subject = $1', [subject])
    return rows[0]?.player_ref
  }
  async forget(playerRef: string): Promise<void> {
    await db.query('DELETE FROM lingara_players WHERE player_ref = $1', [playerRef])
  }
}

const directory = new PlayerTable()
const router = new PlayerRouter({ secrets: process.env.LINGARA_WEBHOOK_SECRET ?? '', directory })

// POST /lingara/webhook. Pass the raw body: the signature covers its bytes.
export async function webhook(request: Request): Promise<Response> {
  const body = new Uint8Array(await request.arrayBuffer())
  const status = await router.handle(body, request.headers, async ({ event, playerRef }) => {
    // Delivery is at least once: skip an event.id you have already seen.
    // Answer within 10 s; enqueue anything slow and return.
    if (playerRef !== null && event.type === 'practice.completed') await practiceJobs.enqueue(playerRef, event.id)
  })
  return new Response(null, { status })
}
// lingara:end
// lingara:begin mint
const lingara = new Lingara({ clientId: process.env.LINGARA_CLIENT_ID, clientSecret: process.env.LINGARA_CLIENT_SECRET })
const server = new EmbedServer(lingara, { directory })

// POST /lingara/token, behind your own sign-in. Rate-limit it per session.
export async function token(request: Request): Promise<Response> {
  const user = await signedInUser(request)
  if (user === null) return new Response(null, { status: 401 })
  const minted = await server.mintToken(user.id, { origin: 'https://www.example.com' })
  return new Response(tokenHandoff(minted), { headers: { 'content-type': 'application/json', 'cache-control': 'no-store' } })
}
// lingara:end
// lingara:begin deletePlayer
// From your own account-deletion path: revokes the player's tokens at
// Lingara, then forgets their rows. Deleting your own data stays yours.
export async function deleteAccount(userId: string): Promise<void> {
  await server.deletePlayer(userId)
}
// lingara:end
// lingara:begin widgetSnippet
// The page that mounts the widget: the token never appears in it.
export function practicePage(planId: string, cspNonce: string): string {
  const widget = widgetSnippet({ planId, tokenEndpoint: '/lingara/token', theme: 'auto', locale: 'fr', title: 'Pratique', nonce: cspNonce })
  return `<!doctype html><title>Practice</title>${widget}`
}
// lingara:end