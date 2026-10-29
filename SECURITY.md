# Security

## Reporting a vulnerability

Email **support@getlingara.com** with "Security" in the subject line, and
describe the issue and the steps to reproduce it. Please do not open a public
issue for a vulnerability.

## No secret on a player's device

A player kit holds an embed token (`lgr_et_…`), and nothing else. Your OAuth
client secret, its access tokens and your webhook signing secret belong on
your server only. Anything in a game binary or a web page is public: treat it
as published.

An embed token is bound to one player, lives 900 seconds and is never
refreshed by Lingara. A token minted for the web is bound to one origin.

The kits never log a secret, a token or a signature, at any log level. A kit
that does is a vulnerability, and we want to hear about it.

## Release signing

The six Maven artefacts at `maven.getlingara.com`, under `com.getlingara:lingara-embed-*`, are
signed with the Lingara release key,
`Lingara Release Signing <support@getlingara.com>`, whose fingerprint is:

    084F59E770709FC8C082A8F0C5EE6D0DB337D775

The public key is on `keyserver.ubuntu.com` and `keys.openpgp.org`. A
signature by any other key did not come from us. The crates, npm, NuGet and
RubyGems packages are published from this repository's release workflow
through each registry's trusted publishing, and npm's carry provenance. Every
GitHub release lists a `checksums.txt` of its assets.
