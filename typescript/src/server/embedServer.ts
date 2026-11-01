// Mint and delete (ADR 1.10.26af D4, D9): one library call each, the
// directory written only after a success. There is no cache: every
// `mintToken` is one `POST`, so a token `deletePlayer` revoked on another
// instance is never handed out again.

import type { EmbedTokenRequest, Lingara, MintedToken, WebhookHeaders } from '@lingara/api'

import type { PlayerDirectory } from './directory.js'

export interface EmbedServerOptions {
  /** Records each mint's subject, and forgets a deleted player. Optional: you may store `minted.subject` yourself. */
  directory?: PlayerDirectory | undefined
}

/**
 * What a mint may send beside the player: each optional, and sent only when
 * given. `origin` is your page's origin, one of your client's embed origins;
 * a game's server mints without one. `embedVersion` is the player build's
 * declared version, read with {@link embedVersionFrom}; Lingara answers an
 * outdated one with `minted.updateAvailable`. It is not validated here:
 * Lingara is the authority.
 */
export interface MintOptions {
  scopes?: readonly string[] | undefined
  origin?: string | undefined
  embedVersion?: string | undefined
  signal?: AbortSignal | undefined
}

/** The header a player kit names its build's embed version in (ADR 30.10.26p D1). */
const EMBED_VERSION_HEADER = 'lingara-embed-version'

/**
 * The `Lingara-Embed-Version` header a player kit sends to your token route,
 * read case-insensitively and trimmed (ADR 31.10.26f D1). A missing or blank
 * header is `undefined`, so the mint sends no key. Takes a fetch `Headers` or
 * a Node-style header record, as `PlayerRouter.handle` does.
 */
export function embedVersionFrom(headers: WebhookHeaders): string | undefined {
  const value = headerValue(headers, EMBED_VERSION_HEADER)?.trim()
  return value === undefined || value === '' ? undefined : value
}

function headerValue(headers: WebhookHeaders, name: string): string | undefined {
  // Duck-typed like the library's own lookup: a framework's Headers may not be the global class.
  if (isFetchHeaders(headers)) return headers.get(name) ?? undefined
  const entry = Object.entries(headers).find(([key]) => key.toLowerCase() === name)
  const value = entry?.[1]
  // A repeated header (Node's array form) reads as its first value.
  return typeof value === 'string' || value === undefined ? value : value[0]
}

function isFetchHeaders(headers: WebhookHeaders): headers is Headers {
  return typeof (headers as Partial<Headers>).get === 'function'
}

export interface DeleteOptions {
  signal?: AbortSignal | undefined
}

type Scopes = NonNullable<EmbedTokenRequest['scopes']>

/** Holds your `@lingara/api` client and, optionally, your player directory. */
export class EmbedServer {
  readonly #client: Lingara
  readonly #directory: PlayerDirectory | undefined

  /** Over a client built with your client credentials. Its token cache, retries and deprecation hook are the library's. */
  constructor(client: Lingara, options: EmbedServerOptions = {}) {
    this.#client = client
    this.#directory = options.directory
  }

  /**
   * Mints a token for one player: one `POST /v1/embed/tokens`, then
   * `remember(playerRef, subject)`. On a failure nothing is remembered and
   * the library's error is raised unchanged.
   */
  async mintToken(playerRef: string, options: MintOptions = {}): Promise<MintedToken> {
    const body: EmbedTokenRequest = { player_ref: playerRef }
    if (options.scopes !== undefined) body.scopes = [...options.scopes] as Scopes
    if (options.origin !== undefined) body.origin = options.origin
    if (options.embedVersion !== undefined) body.embed_version = options.embedVersion
    const minted: MintedToken = await this.#client.createEmbedToken(body, { signal: options.signal })
    await this.#directory?.remember(playerRef, minted.subject)
    return minted
  }

  /**
   * Deletes a player and revokes their tokens, then forgets them. An unknown
   * player is a success; on a failure the row is kept.
   */
  async deletePlayer(playerRef: string, options: DeleteOptions = {}): Promise<void> {
    await this.#client.deleteEmbedPlayer({ playerRef }, { signal: options.signal })
    await this.#directory?.forget(playerRef)
  }

  toJSON(): Record<string, unknown> {
    return { client: this.#client.toJSON(), directory: this.#directory !== undefined }
  }

  toString(): string {
    return `EmbedServer ${JSON.stringify(this.toJSON())}`
  }
}
