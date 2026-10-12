# The libGDX example

A minimal libGDX game over the Kotlin kit: one screen with a scene change, a
practice session and an NPC whose reply streams into a speech bubble. It
builds against this repository's own code, never a released package.

Keys: **S** walks to the other stall (a scene change), **P** starts a
practice, **1–4** choose an option, type and **Enter** to answer, **N**
reveals or goes on, **T** says a line to Mei, **L** switches her level
between 2 and 4.

The default libGDX font draws Latin text only, so Chinese prompts show as
boxes; a real game ships its own font.

## Run it

It takes its token from the demo token server, which mints for one demo
player from a metered client's credentials:

```sh
LINGARA_CLIENT_ID=… LINGARA_CLIENT_SECRET=… cargo run -p lingara_demo_token_server
./gradlew -p kotlin/examples/libgdx :lwjgl3:run
```

With no Lingara account, run the token server with `--fake` and point the
game at the conformance server instead:

```sh
cargo run -p lingara_demo_token_server -- --fake
cargo run -p lingara-embed-conformance-server -- serve --cases conformance/ek/cases   # prints `listening <port>`
LINGARA_EMBED_BASE_URL=http://127.0.0.1:<port> ./gradlew -p kotlin/examples/libgdx :lwjgl3:run
```

`LINGARA_DEMO_TOKEN_URL` points the game at another token endpoint, and
`LINGARA_DEMO_EXIT_AFTER_SECS` exits after that many seconds.

## On Android

The token server binds loopback only, so first forward the port, on an
emulator or a device alike:

```sh
adb reverse tcp:8787 tcp:8787
./gradlew -p kotlin/examples/libgdx :android:installRelease
```

The release build is minified, so it also checks the natives artefact's R8
rules. The kit ships arm64-v8a only: use an arm64 device or emulator image.
