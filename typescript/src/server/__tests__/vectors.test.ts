// The shared vectors (ADR 1.10.26af ES4, ES6), read from the repository's
// conformance/vectors, the way the libraries read webhook-signatures.json.

import { readFileSync } from 'node:fs'

import { describe, expect, it } from 'vitest'

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
  input: { token: string; expires_at: string }
  body: string
}

describe('1.10.26af — the snippet and handoff vectors', () => {
  /** 1.10.26af AC12: every snippet.json case renders its exact html or refuses its field; every token-handoff.json case is its exact body. */
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
      expect(handoffBody(vector.input.token, vector.input.expires_at), vector.name).toBe(vector.body)
    }
  })
})
