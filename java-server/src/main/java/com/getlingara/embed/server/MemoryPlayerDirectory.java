package com.getlingara.embed.server;

import java.util.Map;
import java.util.Optional;
import java.util.TreeMap;

/**
 * A process-local {@link PlayerDirectory}, for tests and single-process demos. It is lost on
 * restart and not shared between instances: production code implements {@link PlayerDirectory} over
 * its own database. It is safe for concurrent use.
 */
public final class MemoryPlayerDirectory implements PlayerDirectory {
  private final Map<String, String> rows = new TreeMap<>();

  /** An empty directory. */
  public MemoryPlayerDirectory() {
    // Rows arrive through remember().
  }

  @Override
  public synchronized void remember(String playerRef, String subject) {
    rows.put(subject, playerRef);
  }

  @Override
  public synchronized Optional<String> playerRefFor(String subject) {
    return Optional.ofNullable(rows.get(subject));
  }

  @Override
  public synchronized void forget(String playerRef) {
    rows.values().removeIf(playerRef::equals);
  }

  /**
   * Returns every row, subject to player_ref, sorted by subject.
   *
   * @return a copy of the rows
   */
  public synchronized Map<String, String> snapshot() {
    return new TreeMap<>(rows);
  }

  @Override
  public synchronized String toString() {
    return "MemoryPlayerDirectory{rows=" + rows.size() + "}";
  }
}
