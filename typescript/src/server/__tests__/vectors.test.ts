// The shared vectors (ADR 1.10.26af ES4, ES6), read from the repository's
// conformance/vectors, the way the libraries read webhook-signatures.json.

import { readFileSync } from 'node:fs'

import { describe, expect, it } from 'vitest'

import type { UpdateAvailable } from '@lingara/api'

import { handoffBody } from '../handoff.js'
import { SnippetError, VERSION, widgetSnippet, type SnippetOptions } from '../widgetSnippet.js'

const repo = new URL('../../../../', import.meta.url)
const read = (path: string): string => readFileSync(new URL(path, repo), 'utf8')

interface SnippetVector {
  name: string
  options: SnippetOptions
  expect: { html: string } | { refused: string }
}

interface HandoffVector {
  name: string
  input: { token: string; expires_at: string; update_available?: Record<string, unknown> }
  body: string
}

/** `input.update_available` decoded field by field into the typed value `MintedToken.updateAvailable` carries (31.10.26f D1 item 4). */
function notice(raw: Record<string, unknown> | undefined): UpdateAvailable | undefined {
  if (raw === undefined) return undefined
  return { message: raw['message'] as string | null, deprecated: raw['deprecated'] as boolean, latest: raw['latest'] as string | null }
}

describe('1.10.26af — the snippet and handoff vectors', () => {
  /** 1.10.26af AC12: every snippet.json case renders its exact html or refuses its field; every token-handoff.json case is its exact body, update_available included. */
  // 31.10.26f AC4
  it('every snippet and handoff vector', () => {
    expect(VERSION).toBe(read('VERSION').trim())

    const snippets: SnippetVector[] = JSON.parse(read('conformance/vectors/snippet.json')).cases
    expect(snippets.length).toBeGreaterThan(0)
    for (const vector of snippets) {
      if ('html' in vector.expect) {
        expect(widgetSnippet(vector.options), vector.name).toBe(vector.expect.html.replaceAll('{{version}}', VERSION))
      } else {
        let refused: unknown
        try {
          widgetSnippet(vector.options)
        } catch (error) {
          refused = error
        }
        expect(refused, vector.name).toBeInstanceOf(SnippetError)
        expect((refused as SnippetError).field, vector.name).toBe(vector.expect.refused)
      }
    }

    const handoffs: HandoffVector[] = JSON.parse(read('conformance/vectors/token-handoff.json'))
    expect(handoffs.length).toBeGreaterThan(0)
    for (const vector of handoffs) {
      expect(handoffBody(vector.input.token, vector.input.expires_at, notice(vector.input.update_available)), vector.name).toBe(vector.body)
    }
  })
})
