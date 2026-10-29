# The Godot demo

A night market with two stall-keepers: **Mei** sells dumplings and speaks
Mandarin at level 2; **Professor Tanaka** keeps a bookstall and speaks
Japanese at level 6. Click a stall to talk to its keeper, type a line (or
take a preset), change their level, walk between the stalls, and practise
over a lesson plan Lingara makes for you. It is the smallest working
GDScript game on the addon, and unpublished.

## Running it

Three terminals, from the repository root.

1. **The token server.** It stands in for your game's server and needs a
   **metered** client's credentials (only a metered client may mint player
   tokens):

   ```sh
   LINGARA_CLIENT_ID=… LINGARA_CLIENT_SECRET=… cargo run -p lingara_demo_token_server
   ```

   Against a local stack, add `LINGARA_BASE_URL=http://localhost:8080` here
   and `LINGARA_DEMO_API_URL=http://localhost:8080` to the demo below.

2. **The demo,** in the editor (press F5 to run it):

   ```sh
   make godot-demo GODOT=/path/to/Godot
   ```

   It asks `http://127.0.0.1:8787/token` for tokens (`token_client.gd`, the
   `token_needed` round trip) and talks to production.
   `LINGARA_DEMO_TOKEN_URL` and `LINGARA_DEMO_API_URL` point it elsewhere. On
   an Android device, `adb reverse tcp:8787 tcp:8787` reaches the token
   server.

3. **A webhook receiver,** to watch the practice report arrive as
   `practice.completed`: run your server kit's `PlayerRouter` example behind a
   tunnel, and register its URL as your client's webhook.

`project.godot` sets `lingara/autostart` and `lingara/stream_on_start` false:
`main.gd` sets `base_url` and then calls `Lingara.start()`, so no request
leaves before the demo's configuration is in place. The demo reads no stream
event itself; a plan still being made turns the stream on by itself.

The text size is the demo's own: `project.godot`'s `gui/theme/custom` names
`demo_theme.tres`, whose `default_font_size` (24) reaches every control,
including the practice options `practice_panel.gd` builds at runtime
(ADR 15.10.26f). The addon sets no theme; your game keeps its own.

## The tests

Both run headless, against `conformance-server`, with the addon copied in by
`make godot-demo-addon`:

- `make conformance-ek-godot` runs `tests/ek_runner.gd`, the EK harness:
  every EK case through `LingaraEmbed`.
- `make godot-headless` runs `tests/headless_demo.gd` against
  `tests/cases/demo/talk-and-practise.yaml` and the token server's `--fake`
  mode: three turns at levels 2, 6 and 3, and a practice that reports once.

`tests/` is excluded from every export preset.
