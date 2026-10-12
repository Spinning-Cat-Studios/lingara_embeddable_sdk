package com.getlingara.embed.server.kotlin

import com.getlingara.kotlin.events.Webhook
import com.getlingara.kotlin.events.WebhookVerificationException
import kotlin.coroutines.cancellation.CancellationException

/**
 * Routes a Lingara webhook to its player (ADR 1.10.26af D8): the library's verifier on the raw body
 * first, then the [directory], so a forged event never reaches your store. [handle] maps every
 * outcome to the status your route sends, the same in every server kit:
 *
 * | Outcome | Status |
 * |---|---|
 * | `onEvent` returned | 204 |
 * | a bad signature (five reasons) | 401 |
 * | a valid signature over a malformed payload | 400 |
 * | `onEvent` or the directory threw | 500, the cause to [onError] |
 *
 * Every status has an empty body. [onError] defaults to an error line on
 * `System.getLogger("com.getlingara.embed.server.kotlin")`.
 */
public class PlayerRouter(
    private val directory: PlayerDirectory,
    private val webhook: Webhook,
    private val onError: ((Throwable) -> Unit)? = null,
) {
    /**
     * A router over your webhook secrets: one, or two while a rotation has both live. The verifier
     * is built here, once, so a malformed secret fails at startup.
     *
     * @throws IllegalArgumentException for no secret, or a secret not of that shape
     */
    public constructor(
        directory: PlayerDirectory,
        vararg secrets: String,
        onError: ((Throwable) -> Unit)? = null,
    ) : this(directory, Webhook(*secrets), onError)

    /**
     * Verifies the **raw** body, then looks the subject up.
     *
     * @throws WebhookVerificationException before the directory is consulted
     */
    public suspend fun route(
        body: ByteArray,
        headers: Map<String, List<String>>,
    ): RoutedEvent {
        val event = webhook.verify(body, headers)
        return RoutedEvent(event, directory.playerRefFor(event.subject))
    }

    /** [route], then [onEvent], answered as the status your route sends; the body is always empty. */
    public suspend fun handle(
        body: ByteArray,
        headers: Map<String, List<String>>,
        onEvent: suspend (RoutedEvent) -> Unit,
    ): Int {
        val routed =
            try {
                route(body, headers)
            } catch (e: WebhookVerificationException) {
                return if (e.reason == WebhookVerificationException.Reason.MALFORMED_PAYLOAD) 400 else 401
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                return failed("the player directory failed", e)
            }
        return try {
            onEvent(routed)
            204
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            failed("the event handler failed", e)
        }
    }

    override fun toString(): String = "PlayerRouter(webhook=$webhook, directory=$directory)"

    private fun failed(
        what: String,
        e: Exception,
    ): Int {
        onError?.invoke(e) ?: LOG.log(System.Logger.Level.ERROR, "lingara webhook: $what", e)
        return 500
    }

    private companion object {
        val LOG: System.Logger = System.getLogger("com.getlingara.embed.server.kotlin")
    }
}
