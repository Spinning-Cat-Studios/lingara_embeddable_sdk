# lingara-embed-natives-desktop

The native library behind [`lingara-embed-kotlin`](../kotlin/README.md) and
[`lingara-embed-java`](../java/README.md) on the desktop, for Windows x64,
macOS (arm64 and x64, 11.0+) and Linux x64 (glibc 2.28+), each under JNA's
resource prefix so JNA finds and extracts it itself. It also brings JNA's
desktop JAR.

Add it beside the kit, at the same version: in a libGDX project, in the
`lwjgl3` module. The JAR carries all four libraries, so a game that ships to
one desktop OS still downloads them all at build time.

`jna.library.path` still wins over it, for local development against a
library you built yourself.
