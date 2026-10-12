// The host half against the protocol's shared vectors.
//
// SOURCE: protocol-v1.json is a byte-identical copy of the hosted page's
// web/src/embed/__tests__/protocolVectors.json at the Lingara web app's commit
// 7903f4191. A protocol change edits that file first, then this copy.

import { describe, expect, it } from 'vitest'

import { ERROR_CODES, FRAME_TYPES, HOST_TYPES, hostMessage, parseFrameMessage, type HostPayloads, type HostType } from '../protocol.js'
import vectors from './protocol-v1.json'

describe('1.10.26ad — the protocol, host side', () => {
  it('1.10.26ad AC10 the host half agrees with every protocol vector', () => {
    for (const [type, vector] of Object.entries(vectors.frame)) {
      expect(parseFrameMessage(vector.valid), type).toMatchObject({ type, payload: (vector.valid as { payload: object }).payload })
      expect(parseFrameMessage(vector.ignored), `${type} ignored`).toBeNull()
    }
    for (const [type, vector] of Object.entries(vectors.host)) {
      const { payload } = vector.parsed as { payload: HostPayloads[HostType] }
      expect(hostMessage(type as HostType, payload), type).toEqual(vector.valid)
    }

    // The vectors name every closed set, so this copy is held to all of them.
    expect(Object.keys(vectors.host).sort()).toEqual([...HOST_TYPES].sort())
    expect(Object.keys(vectors.frame).sort()).toEqual([...FRAME_TYPES].sort())
    expect(ERROR_CODES).toHaveLength(12)
    expect(ERROR_CODES).toContain(vectors.frame.error.valid.payload.code)
  })

  it('1.10.26ad AC10 an unknown type, a missing field or another version is ignored', () => {
    const started = vectors.frame.started.valid
    for (const junk of ['ready', 42, null, [], { ...started, type: 'shutdown' }, { ...started, payload: { surface: 'practice' } }]) {
      expect(parseFrameMessage(junk)).toBeNull()
    }
    expect(parseFrameMessage({ ...started, version: 2 })).toBeNull()
    // `ready` is the negotiation, so it is read at any version.
    const ready = { ...vectors.frame.ready.valid, version: 2, payload: { versions: [2], surfaces: ['practice'] } }
    expect(parseFrameMessage(ready)).toEqual({ type: 'ready', payload: { versions: [2], surfaces: ['practice'] } })
    // A newer frame's extra field goes no further than the parser.
    const extra = { ...vectors.frame.resized.valid, payload: { height: 300, secret: 'x' } }
    expect(parseFrameMessage(extra)).toEqual({ type: 'resized', payload: { height: 300 } })
  })
})
