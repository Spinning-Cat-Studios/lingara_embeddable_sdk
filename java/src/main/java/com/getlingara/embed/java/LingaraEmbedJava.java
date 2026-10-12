package com.getlingara.embed.java;

import com.getlingara.embed.Command;
import com.getlingara.embed.CommandOutcome;
import com.getlingara.embed.DialogueConfig;
import com.getlingara.embed.DialogueSnapshot;
import com.getlingara.embed.EmbedConfig;
import com.getlingara.embed.EmbedEvent;
import com.getlingara.embed.EmbedException;
import com.getlingara.embed.EmbedStatus;
import com.getlingara.embed.Header;
import com.getlingara.embed.KitSurface;
import com.getlingara.embed.LingaraEmbed;
import com.getlingara.embed.LingaraEmbedCommands;
import com.getlingara.embed.LingaraEmbedDialogue;
import com.getlingara.embed.LingaraEmbedPractice;
import com.getlingara.embed.LogLevel;
import com.getlingara.embed.PracticeSnapshot;
import com.getlingara.embed.PracticeSource;
import com.getlingara.embed.Secret;
import com.getlingara.embed.TokenFetcher;
import com.getlingara.embed.TokenSourceConfig;
import com.getlingara.embed.WorldContext;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.function.Consumer;

/**
 * One player's connection to Lingara, for Java 17 (ADR 1.10.26ae D4). Build it once, drain it once
 * a frame on the game thread, close it in {@code dispose()} or a try-with-resources block.
 *
 * <pre>{@code
 * try (LingaraEmbedJava lingara = LingaraEmbedJava.builder()
 *     .httpTokenSource("https://my-game.example/lingara/token", Map.of("Authorization", bearer))
 *     .userAgentSuffix("my-game/1.4")
 *     .build()) {
 *   lingara.drain(256, event -> LingaraEmbedJava.visit(event, myVisitor));
 * }
 * }</pre>
 *
 * <p>Every method may be called from any thread. A refusal is a {@link LingaraEmbedJavaException};
 * no method declares a checked exception.
 */
public final class LingaraEmbedJava implements AutoCloseable {
  private final LingaraEmbed embed;

  private LingaraEmbedJava(LingaraEmbed embed) {
    this.embed = embed;
  }

  /** A builder over {@link EmbedConfig}: every field left unset takes the core's default. */
  public static Builder builder() {
    return new Builder();
  }

  /** The next event, or empty when the queue is empty. */
  public Optional<EmbedEvent> poll() {
    return Optional.ofNullable(embed.poll());
  }

  /** At most {@code max} events, in queue order, in one crossing. */
  public List<EmbedEvent> drain(int max) {
    return embed.drain(max);
  }

  /** {@link #drain(int)}, handing each event to {@code handler}. */
  public void drain(int max, Consumer<EmbedEvent> handler) {
    embed.drain(max).forEach(handler);
  }

  /** Exhaustive dispatch of one event to the visitor's method for its subclass. */
  public static <R> R visit(EmbedEvent event, EmbedEventVisitor<R> visitor) {
    return Dispatch.visit(event, visitor);
  }

  public CommandOutcome execute(Command command) {
    return Calls.call(() -> embed.execute(command));
  }

  public EmbedStatus status() {
    return Calls.call(embed::status);
  }

  public void provideToken(String token, String expiresAt) {
    Calls.run(() -> LingaraEmbedCommands.provideToken(embed, new Secret(token), expiresAt));
  }

  public void provideTokenFor(String token, long expiresInSecs) {
    Calls.run(() -> LingaraEmbedCommands.provideTokenFor(embed, new Secret(token), expiresInSecs));
  }

  public void tokenUnavailable(String reason) {
    Calls.run(() -> LingaraEmbedCommands.tokenUnavailable(embed, reason));
  }

  /** Re-points the Http token source; the headers in the map's iteration order. */
  public void setTokenEndpoint(String url, Map<String, String> headers) {
    Calls.run(() -> LingaraEmbedCommands.setTokenEndpoint(embed, url, headers(headers)));
  }

  public void streamPause() {
    Calls.run(() -> LingaraEmbedCommands.streamPause(embed));
  }

  public void streamResume() {
    Calls.run(() -> LingaraEmbedCommands.streamResume(embed));
  }

  public long sendContext(WorldContext context) {
    return Calls.call(() -> LingaraEmbedCommands.sendContext(embed, context));
  }

  public long practiceStart(PracticeSource source) {
    return Calls.call(() -> LingaraEmbedPractice.practiceStart(embed, source));
  }

  public Optional<PracticeSnapshot> practiceSnapshot(long session) {
    return Optional.ofNullable(
        Calls.call(() -> LingaraEmbedPractice.practiceSnapshot(embed, session)));
  }

  public PracticeSnapshot practiceAnswer(long session, String answer) {
    return Calls.call(() -> LingaraEmbedPractice.practiceAnswer(embed, session, answer));
  }

  public PracticeSnapshot practiceChoose(long session, long option) {
    return Calls.call(() -> LingaraEmbedPractice.practiceChoose(embed, session, option));
  }

  public PracticeSnapshot practiceReveal(long session) {
    return Calls.call(() -> LingaraEmbedPractice.practiceReveal(embed, session));
  }

  public PracticeSnapshot practiceSelfGrade(long session, boolean correct) {
    return Calls.call(() -> LingaraEmbedPractice.practiceSelfGrade(embed, session, correct));
  }

  public PracticeSnapshot practiceNext(long session) {
    return Calls.call(() -> LingaraEmbedPractice.practiceNext(embed, session));
  }

  public PracticeSnapshot practiceFinish(long session) {
    return Calls.call(() -> LingaraEmbedPractice.practiceFinish(embed, session));
  }

  public long dialogueOpen(DialogueConfig config) {
    return Calls.call(() -> LingaraEmbedDialogue.dialogueOpen(embed, config));
  }

  public long dialogueSay(long dialogue, String line) {
    return Calls.call(() -> LingaraEmbedDialogue.dialogueSay(embed, dialogue, line));
  }

  public void dialogueSetLevel(long dialogue, int level) {
    Calls.run(() -> LingaraEmbedDialogue.dialogueSetLevel(embed, dialogue, level));
  }

  /** {@code scene} may be null: the conversation then has no scene. */
  public void dialogueSetScene(long dialogue, String scene) {
    Calls.run(() -> LingaraEmbedDialogue.dialogueSetScene(embed, dialogue, scene));
  }

  public void dialogueCancel(long dialogue) {
    Calls.run(() -> LingaraEmbedDialogue.dialogueCancel(embed, dialogue));
  }

  public void dialogueClose(long dialogue) {
    Calls.run(() -> LingaraEmbedDialogue.dialogueClose(embed, dialogue));
  }

  public Optional<DialogueSnapshot> dialogueSnapshot(long dialogue) {
    return Optional.ofNullable(
        Calls.call(() -> LingaraEmbedDialogue.dialogueSnapshot(embed, dialogue)));
  }

  /**
   * Signals the core to stop and returns at once, keeping the handle: the queue then yields one
   * {@code Closed}. {@link #close()} still frees it.
   */
  public void shutdown() {
    embed.shutdown();
  }

  /** Stops the core, waits at most 1 s for its thread, and frees the handle. Idempotent. */
  @Override
  public void close() {
    embed.close();
  }

  /** {@link #close()} with its own bound: true when the core's thread joined in time. */
  public boolean closeAndWait(long timeoutMs) {
    return embed.closeAndWait(timeoutMs);
  }

  private static List<Header> headers(Map<String, String> headers) {
    List<Header> out = new ArrayList<>(headers.size());
    headers.forEach((name, value) -> out.add(new Header(name, new Secret(value))));
    return out;
  }

  /** The config Kotlin's default arguments would build. */
  public static final class Builder {
    private String baseUrl;
    private TokenSourceConfig tokenSource;
    private String apiVersion;
    private List<String> eventTypes = List.of();
    private String resumeCursor;
    private Boolean streamOnStart;
    private Long queueCapacity;
    private Long tokenWaitTimeoutMs;
    private Long streamIdleTimeoutMs;
    private LogLevel logLevel;
    private String userAgentSuffix;
    private TokenFetcher tokenFetcher;

    private Builder() {}

    public Builder baseUrl(String baseUrl) {
      this.baseUrl = baseUrl;
      return this;
    }

    /** The game's own token endpoint; each header value is held as a {@link Secret}. */
    public Builder httpTokenSource(String url, Map<String, String> headers) {
      this.tokenSource = new TokenSourceConfig.Http(url, headers(headers));
      return this;
    }

    /** Answers the poll source's {@code TokenNeeded} from Java, off the game thread. */
    public Builder tokenFetcher(TokenFetcher tokenFetcher) {
      this.tokenFetcher = tokenFetcher;
      return this;
    }

    public Builder apiVersion(String apiVersion) {
      this.apiVersion = apiVersion;
      return this;
    }

    /** The stream's event types; empty, the default, is every type. */
    public Builder eventTypes(List<String> eventTypes) {
      this.eventTypes = List.copyOf(eventTypes);
      return this;
    }

    public Builder queueCapacity(long queueCapacity) {
      this.queueCapacity = queueCapacity;
      return this;
    }

    public Builder tokenWaitTimeoutMs(long tokenWaitTimeoutMs) {
      this.tokenWaitTimeoutMs = tokenWaitTimeoutMs;
      return this;
    }

    /** A heartbeat gap this long reconnects the stream (at least 1 000). */
    public Builder streamIdleTimeoutMs(long streamIdleTimeoutMs) {
      this.streamIdleTimeoutMs = streamIdleTimeoutMs;
      return this;
    }

    public Builder resumeCursor(String resumeCursor) {
      this.resumeCursor = resumeCursor;
      return this;
    }

    /** {@code false} starts paused: a game that never listens pays nothing. */
    public Builder streamOnStart(boolean streamOnStart) {
      this.streamOnStart = streamOnStart;
      return this;
    }

    public Builder logLevel(LogLevel logLevel) {
      this.logLevel = logLevel;
      return this;
    }

    /** Appended after {@code lingara-embed-java/<version>}, e.g. {@code my-game/1.4}. */
    public Builder userAgentSuffix(String userAgentSuffix) {
      this.userAgentSuffix = userAgentSuffix;
      return this;
    }

    /** Loads the natives and starts the core. */
    public LingaraEmbedJava build() {
      EmbedConfig config =
          new EmbedConfig(
              baseUrl,
              tokenSource,
              apiVersion,
              eventTypes,
              resumeCursor,
              streamOnStart,
              queueCapacity,
              tokenWaitTimeoutMs,
              streamIdleTimeoutMs,
              logLevel,
              userAgentSuffix);
      return new LingaraEmbedJava(
          Calls.call(() -> LingaraEmbed.startAs(KitSurface.JAVA, config, tokenFetcher)));
    }
  }

  /** A call into the Kotlin layer, its checked {@code EmbedException} made unchecked. */
  private static final class Calls {
    private Calls() {}

    @FunctionalInterface
    interface Call<T> {
      T call() throws EmbedException;
    }

    @FunctionalInterface
    interface Run {
      void run() throws EmbedException;
    }

    static <T> T call(Call<T> call) {
      try {
        return call.call();
      } catch (EmbedException e) {
        throw new LingaraEmbedJavaException(e);
      }
    }

    static void run(Run run) {
      try {
        run.run();
      } catch (EmbedException e) {
        throw new LingaraEmbedJavaException(e);
      }
    }
  }
}
