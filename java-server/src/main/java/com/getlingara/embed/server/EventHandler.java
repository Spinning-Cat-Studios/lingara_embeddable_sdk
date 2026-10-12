package com.getlingara.embed.server;

/**
 * What {@link PlayerRouter#handle} calls with each verified, routed event. It runs inline, and
 * Lingara expects a {@code 2xx} within 10 s, so a slow handler enqueues and returns. Delivery is at
 * least once: deduplicate on {@code routed.event().id()}.
 */
@FunctionalInterface
public interface EventHandler {
  /**
   * Handles one event. Throwing answers {@code 500}, and Lingara redelivers.
   *
   * @param routed the event and the player it concerns
   * @throws Exception anything; it reaches the router's {@code onError}, never Lingara
   */
  void onEvent(RoutedEvent routed) throws Exception;
}
