package com.getlingara.embed.server.conformance;

import com.getlingara.embed.server.MemoryPlayerDirectory;
import com.getlingara.embed.server.PlayerDirectory;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.logging.Handler;
import java.util.logging.Level;
import java.util.logging.LogRecord;
import java.util.logging.Logger;

/**
 * The case's {@link MemoryPlayerDirectory} with its {@code playerRefFor} calls counted: a lookup
 * changes no contents, so only a count shows a forged delivery never reached the store.
 */
final class Recording implements PlayerDirectory {
  private final MemoryPlayerDirectory inner = new MemoryPlayerDirectory();
  private final AtomicInteger lookups = new AtomicInteger();

  Recording(Map<String, String> seed) {
    seed.forEach((subject, player) -> inner.remember(player, subject));
  }

  int lookups() {
    return lookups.get();
  }

  Map<String, String> snapshot() {
    return inner.snapshot();
  }

  @Override
  public void remember(String playerRef, String subject) {
    inner.remember(playerRef, subject);
  }

  @Override
  public Optional<String> playerRefFor(String subject) {
    lookups.incrementAndGet();
    return inner.playerRefFor(subject);
  }

  @Override
  public void forget(String playerRef) {
    inner.forget(playerRef);
  }

  @Override
  public String toString() {
    return "Recording{" + inner + "}";
  }

  /** Every line the routers' {@code onError} and {@code System.Logger} wrote. */
  static final class Sink {
    private final List<String> lines = new ArrayList<>();

    synchronized void push(String line) {
      lines.add(line);
    }

    synchronized List<String> take() {
      List<String> out = List.copyOf(lines);
      lines.clear();
      return out;
    }

    /**
     * Routes the root java.util.logging logger, where {@code System.getLogger} lands by default,
     * into the sink: the router's default {@code onError} facility.
     */
    void captureLogs() {
      Logger root = Logger.getLogger("");
      for (Handler h : root.getHandlers()) {
        root.removeHandler(h);
      }
      root.setLevel(Level.ALL);
      root.addHandler(
          new Handler() {
            @Override
            public void publish(LogRecord r) {
              Throwable t = r.getThrown();
              push(
                  r.getLevel()
                      + " "
                      + r.getLoggerName()
                      + ": "
                      + r.getMessage()
                      + (t == null ? "" : " " + t));
            }

            @Override
            public void flush() {
              // Nothing is buffered.
            }

            @Override
            public void close() {
              // Nothing is held.
            }
          });
    }
  }
}
