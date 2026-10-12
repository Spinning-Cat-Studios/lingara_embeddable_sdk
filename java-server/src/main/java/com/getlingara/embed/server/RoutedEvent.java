package com.getlingara.embed.server;

import com.getlingara.client.events.Event;
import java.util.Objects;
import java.util.Optional;

/**
 * A verified event and the player it concerns (ADR 1.10.26af D8). {@code playerRef} is empty for a
 * subject the directory does not know: your own subject, or a deleted player. An event newer than
 * the library arrives as an {@code UnknownEvent} and is routed like any other.
 *
 * @param event the library's event
 * @param playerRef the player, if the directory knows the subject
 */
public record RoutedEvent(Event event, Optional<String> playerRef) {
  /**
   * Checks both components are present.
   *
   * @param event the library's event
   * @param playerRef the player, if the directory knows the subject
   */
  public RoutedEvent {
    Objects.requireNonNull(event, "event");
    Objects.requireNonNull(playerRef, "playerRef");
  }
}
