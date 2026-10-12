package com.getlingara.embed.server;

import com.getlingara.client.events.Event;
import com.getlingara.client.events.Webhook;
import com.getlingara.client.events.WebhookVerificationException;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.function.BiConsumer;
import java.util.function.Consumer;

/**
 * Routes a Lingara webhook to its player (ADR 1.10.26af D8): the library's verifier on the raw body
 * first, then the directory, so a forged event never reaches your store. {@link #handle} maps every
 * outcome to the status your route sends, the same in every server kit:
 *
 * <table>
 *   <caption>handle's answers</caption>
 *   <tr><th>Outcome</th><th>Status</th></tr>
 *   <tr><td>the handler returned</td><td>204</td></tr>
 *   <tr><td>a bad signature (five reasons)</td><td>401</td></tr>
 *   <tr><td>a valid signature over a malformed payload</td><td>400</td></tr>
 *   <tr><td>the handler or the directory threw</td><td>500, the cause to onError</td></tr>
 * </table>
 *
 * <p>Every status has an empty body. Immutable and safe for concurrent use when the directory is.
 */
public final class PlayerRouter {
  private static final System.Logger LOG = System.getLogger("com.getlingara.embed.server");
  private static final BiConsumer<String, Exception> LOG_IT =
      (what, e) -> LOG.log(System.Logger.Level.ERROR, "lingara webhook: " + what, e);

  private final Webhook webhook;
  private final PlayerDirectory directory;
  private final BiConsumer<String, Exception> onError;

  private PlayerRouter(
      Webhook webhook, PlayerDirectory directory, BiConsumer<String, Exception> onError) {
    this.webhook = Objects.requireNonNull(webhook, "webhook");
    this.directory = Objects.requireNonNull(directory, "directory");
    this.onError = onError;
  }

  /**
   * A router over your webhook secrets: one, or two while a rotation has both live. The verifier is
   * built here, once, so a malformed secret fails at startup.
   *
   * @param directory your player directory
   * @param secrets each {@code lgr_whsec_…}
   * @return the router
   * @throws IllegalStateException for no secret, or a secret not of that shape
   */
  public static PlayerRouter of(PlayerDirectory directory, String... secrets) {
    return new PlayerRouter(Webhook.of(secrets), directory, LOG_IT);
  }

  /**
   * A router over a verifier you built, such as {@code Webhook.of(secrets, clock)}.
   *
   * @param directory your player directory
   * @param webhook the library's verifier
   * @return the router
   */
  public static PlayerRouter of(PlayerDirectory directory, Webhook webhook) {
    return new PlayerRouter(webhook, directory, LOG_IT);
  }

  /**
   * Returns this router with {@code onError} receiving each {@code 500}'s cause, in place of the
   * default {@code System.getLogger("com.getlingara.embed.server")} error line.
   *
   * @param onError what the handler or the directory threw
   * @return a new router
   */
  public PlayerRouter withOnError(Consumer<? super Exception> onError) {
    Objects.requireNonNull(onError, "onError");
    return new PlayerRouter(webhook, directory, (what, e) -> onError.accept(e));
  }

  /**
   * Verifies the <b>raw</b> body, then looks the subject up.
   *
   * @param body the body exactly as received
   * @param headers the request's headers, any case
   * @return the event and its player
   * @throws WebhookVerificationException before the directory is consulted
   */
  public RoutedEvent route(byte[] body, Map<String, List<String>> headers) {
    Event event = webhook.verify(body, headers);
    Optional<String> playerRef = directory.playerRefFor(event.subject());
    return new RoutedEvent(event, playerRef == null ? Optional.empty() : playerRef);
  }

  /**
   * {@link #route}, then {@code onEvent}, answered as the status your route sends.
   *
   * @param body the body exactly as received
   * @param headers the request's headers, any case
   * @param onEvent your handler
   * @return 204, 400, 401 or 500; the response body is always empty
   */
  public int handle(byte[] body, Map<String, List<String>> headers, EventHandler onEvent) {
    RoutedEvent routed;
    try {
      routed = route(body, headers);
    } catch (WebhookVerificationException e) {
      return e.reason() == WebhookVerificationException.Reason.MALFORMED_PAYLOAD ? 400 : 401;
    } catch (RuntimeException e) {
      return failed("the player directory failed", e);
    }
    try {
      onEvent.onEvent(routed);
      return 204;
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      return failed("the event handler was interrupted", e);
    } catch (Exception e) {
      return failed("the event handler failed", e);
    }
  }

  private int failed(String what, Exception e) {
    onError.accept(what, e);
    return 500;
  }

  @Override
  public String toString() {
    return "PlayerRouter{webhook=" + webhook + ", directory=" + directory + "}";
  }
}
