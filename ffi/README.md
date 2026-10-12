# `ffi/` — the C ABI over `lingara-embed`

`lingara-embed-ffi` builds `lingara_embed` as a dynamic library and a static
library. Its contract is one header, [`include/lingara_embed.h`](include/lingara_embed.h).
Unity, Godot C# (through `Lingara.Embed`), Unreal and any C or C++ caller
load the same nine functions. The crate is not published to crates.io. A
Rust game uses `lingara-embed` directly.

## Messages, not structs

Every call moves one UTF-8 JSON object in the core's own shape. A command
goes in to `lingara_embed_execute`, and its answer comes back in the same
call. One event comes out per `lingara_embed_poll`.
[`schema/messages.schema.json`](schema/messages.schema.json) is the
message set. A new core feature is a new command or event type, never a new
function. Unknown keys are ignored, and an unknown event `type` should be
kept as raw JSON, never treated as a crash.

```c
LingaraEmbed *h = NULL;
const char *config = "{\"stream_on_start\": false}";
lingara_embed_new(LINGARA_EMBED_ABI_VERSION_MAJOR, (const uint8_t *)config, strlen(config), &h);

uint8_t buf[16384];
size_t len;
const char *say = "{\"type\": \"dialogue_say\", \"dialogue\": 4, \"line\": \"ni hao\"}";
int32_t s = lingara_embed_execute(h, (const uint8_t *)say, strlen(say), buf, sizeof buf, &len);
/* OK: {"type":"id","id":7}. Its reply arrives as npc_turn events carrying "request": 7. */

while (lingara_embed_poll(h, buf, sizeof buf, &len) == LINGARA_EMBED_OK) { /* one event per call */ }
lingara_embed_free(h);
```

## Buffers and ownership

No memory crosses the boundary. Every buffer is yours. A call returns
`LINGARA_EMBED_BUFFER_TOO_SMALL` with `*out_len` set to the size it needs.
A `NULL` buffer with `cap` 0 is a size probe.

- **`poll`** keeps the event that did not fit and returns it first on the
  next call, so the queue order is kept.
- **`execute`** has already run the command when the answer does not fit.
  Fetch the answer with `lingara_embed_last_answer` on the same thread. Never
  run the command again, because most commands change state.

There is no string to free. The version string is static.

## Two failure channels

| Status | Meaning | Read |
|---|---|---|
| `OK` (0), `EMPTY` (1), `BUFFER_TOO_SMALL` (2) | The call reached the core | the buffer |
| `REFUSED` (3) | The core refused the command or config | the buffer: the core's `EmbedError` JSON |
| negative | The call never reached the core (`ERR_NULL`, `ERR_INVALID_JSON`, `ERR_UNKNOWN_COMMAND`, `ERR_ABI_MISMATCH`, …) | `lingara_embed_last_error`: short English text |

A failure that happens later, such as a network error, a rejected token or a
server refusal, is an `error` event in the queue. Its `error.request` names
the command it answers. A panic inside the library returns `ERR_PANIC` and
poisons the handle. After that, every call on the handle returns
`ERR_CLOSED`, except `free`. Errors, answers and events never contain a
token.

## Threads

- Every function except `free` may be called from any thread.
- Poll from one thread, usually your engine's main thread.
- Call `execute` and its `last_answer` on the same thread, one after the
  other.
- `free` is the last call on a handle. It joins the core's I/O thread,
  waiting at most 1 s.
- Create one handle per player.

## Tokens

The core raises `token_needed`. Your game asks **your** server, which mints
an embed token. The game then sends `provide_token` (or `token_unavailable`
when your server call fails). **Mint native tokens without `origin`.** A
native game sends no origin header. Never put a client secret in a game. To
let the core fetch tokens itself, set
`config.token_source = {"type": "http", "url": …, "headers": […]}`.

## Versioning

`lingara_embed_abi_version()` returns `(MAJOR << 16) | MINOR`. Refuse a library
whose major differs from your header's. `lingara_embed_new` also takes your
major and returns `ERR_ABI_MISMATCH` without allocating. Adding a function is
a minor bump. Changing or removing a function, or changing a status code's
meaning, is a major bump.

## Building

| Target | |
|---|---|
| `make test-ffi` | clippy, the crate's tests, then the C smoke (`-std=c99 -Wall -Werror -pedantic`) over the header alone |
| `make ffi-header` / `make ffi-schema` | rewrite the committed header and schema; `check-ffi-*` compare them |
| `make ffi-dist RID=<rid>` | one platform into `dist/ffi/<rid>/`: `win-x64`, `osx` (universal), `linux-x64` (glibc 2.28), `android-arm64` (API 23, 16 KB pages), `ios-arm64` (static) |
| `make ffi-deps-check` | each built artefact imports only its platform's allowlist (`tools/ffi-deps/`) |
