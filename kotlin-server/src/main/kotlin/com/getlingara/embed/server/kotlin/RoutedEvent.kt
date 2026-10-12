package com.getlingara.embed.server.kotlin

import com.getlingara.kotlin.events.Event

/**
 * A verified event and the player it concerns (ADR 1.10.26af D8). [playerRef] is null for a
 * subject the directory does not know: your own subject, or a deleted player. An event newer than
 * the library arrives as an `UnknownEvent` and is routed like any other.
 */
public data class RoutedEvent(
    val event: Event,
    val playerRef: String?,
)
