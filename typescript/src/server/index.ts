// @lingara/embed/server (ADR 1.10.26af): the server half of the package.
// It mints a player's token, hands it to the player, routes a verified
// webhook to the player it concerns, and deletes a player, over your
// `@lingara/api` client. `@lingara/api` is an optional peer: install it
// beside this package (`npm i @lingara/embed @lingara/api`); without it this
// entry fails at import, naming the missing package.

export { EmbedServer, type DeleteOptions, type EmbedServerOptions, type MintOptions } from './embedServer.js'
export { HandleFailure, PlayerRouter, type OnError, type PlayerRouterOptions, type RoutedEvent } from './router.js'
export { MemoryPlayerDirectory, type MaybePromise, type PlayerDirectory } from './directory.js'
export { DEFAULT_ELEMENT_SRC, SnippetError, VERSION, widgetSnippet, type SnippetOptions } from './widgetSnippet.js'
export { tokenHandoff } from './handoff.js'
export type { MintedToken, WebhookHeaders } from '@lingara/api'
