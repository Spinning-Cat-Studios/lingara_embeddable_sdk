# lingara-embed-natives-android

The native library behind [`lingara-embed-kotlin`](../kotlin/README.md) and
[`lingara-embed-java`](../java/README.md) on Android: an AAR carrying
`arm64-v8a` (API 23+, 16 KB pages, as Google Play requires from Android 15),
JNA's own AAR, the `INTERNET` permission, and the consumer R8 rules JNA needs.

Add it beside the kit, at the same version: in a libGDX project, in the
`android` module. A minified build needs no rules of its own. The kit ships
arm64 only, so test on an arm64 device or emulator image.
