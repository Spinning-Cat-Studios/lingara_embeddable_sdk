# The Bevy demo

Two stall-keepers at a night market and a tea house: **Mei** sells dumplings
and speaks Mandarin at level 2; **Professor Tanaka** speaks Japanese at
level 6. Talk to either, change their level, walk between them, and practise
over a lesson plan Lingara makes for you. It is the smallest working
`bevy_lingara` game, and unpublished.

## Running it

Three terminals, from the repository root.

1. **The token server.** It stands in for your game's server and needs a
   **metered** client's credentials (only a metered client may mint player
   tokens):

   ```sh
   LINGARA_CLIENT_ID=… LINGARA_CLIENT_SECRET=… cargo run -p lingara_demo_token_server
   ```

2. **The demo:**

   ```sh
   make bevy-demo
   ```

   It asks `http://127.0.0.1:8787/token` for tokens and talks to production.
   `LINGARA_DEMO_TOKEN_URL` and `LINGARA_DEMO_API_URL` point it elsewhere.

3. **A webhook receiver,** to watch the practice report arrive as
   `practice.completed`: run your server kit's `PlayerRouter` example behind a
   tunnel, and register its URL as your client's webhook.

## Keys

| Key | Does |
|---|---|
| `1`–`3` | Say one of three lines to the NPC you are talking to |
| `Tab` | Talk to the other NPC, without walking |
| `W` | Walk to the other stall (one `world.context_changed`) |
| `+` / `-` | Raise or lower this NPC's level |
| `P` | Practise, at this NPC's language and level |
| `1`–`4` | During practice: choose an option |
| `Y` / `N` | During practice: grade yourself |
| `Space` | During practice: reveal, or go on |
| `Esc` | Finish the practice early (it still reports) |

Bevy's built-in font has no Chinese or Japanese glyphs, so each reply is
also written to the log.

## The headless test

`make bevy-demo-headless` runs the same logic with no window, against
`conformance-server serve` and the token server's `--fake` mode, through
`tests/cases/demo/talk-and-practise.yaml`.
