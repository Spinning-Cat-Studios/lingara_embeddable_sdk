# The demo token server

**A stand-in: a real server authenticates the player first.** This one
authenticates nobody. It mints for one demo player, so the demos have a token
route to call while you develop. Your game's own server mints only for a
player it has signed in, and rate-limits its token route per session.

```sh
LINGARA_CLIENT_ID=… LINGARA_CLIENT_SECRET=… cargo run -p lingara_demo_token_server
```

- `POST /token` answers exactly `{"token": "lgr_et_…", "expires_at": "…"}`,
  with `Cache-Control: no-store`. It ignores the request body.
- The client must be **metered**: only a metered client may mint player
  tokens, and any other fails at the first mint.
- Tokens are minted with the five player scopes and **no origin**. A native
  game never sends an origin, so an origin-bound token would be refused on
  every call.
- It binds `127.0.0.1` only, on `--port` (default `8787`; `0` picks a free
  one), and prints `listening <port>` first. An Android device or emulator
  reaches it through `adb reverse tcp:8787 tcp:8787`.
- `--player-ref <ref>` mints for another player (default `demo-player-1`).
- `--fake` mints nothing, needs no credentials and no network, and answers a
  visibly fake token. The headless tests use it.
