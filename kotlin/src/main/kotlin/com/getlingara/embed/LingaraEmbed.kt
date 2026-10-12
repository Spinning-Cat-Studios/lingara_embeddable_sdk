package com.getlingara.embed

import java.util.concurrent.atomic.AtomicBoolean

/**
 * One player's connection to Lingara (ADR 1.10.26ae D3): start it once, drain
 * it once a frame on the game thread, close it in `dispose()`.
 *
 * ```kotlin
 * val lingara = LingaraEmbed.start(EmbedConfig(tokenSource = TokenSourceConfig.Http(url, headers)))
 * lingara.drain { event -> when (event) { is EmbedEvent.Practice -> show(event.snapshot); else -> Unit } }
 * lingara.close()
 * ```
 *
 * Every method may be called from any thread. The core does its I/O on its
 * own thread, so nothing here blocks on the network. The commands are in
 * `LingaraEmbedCommands.kt`, `LingaraEmbedPractice.kt` and
 * `LingaraEmbedDialogue.kt`.
 */
public class LingaraEmbed private constructor(
    internal val handle: EmbedHandle,
    tokenFetcher: TokenFetcher?,
) : AutoCloseable {
    private val fetch = tokenFetcher?.let { TokenFetchThread(it, handle) }
    private val closed = AtomicBoolean(false)

    /** The next event, or null when the queue is empty. A fetched `TokenNeeded` is never returned. */
    public fun poll(): EmbedEvent? {
        while (true) {
            val event = handle.pollEvent() ?: return null
            if (!intercepted(event)) return event
        }
    }

    /** At most [max] events, in queue order, in one crossing. */
    @JvmOverloads
    public fun drain(max: Int = DEFAULT_DRAIN): List<EmbedEvent> = handle.drain(max.toLong()).filterNot(::intercepted)

    /** [drain], handing each event to [handler] with no lambda object. */
    public inline fun drain(
        max: Int = DEFAULT_DRAIN,
        handler: (EmbedEvent) -> Unit,
    ) {
        for (event in drain(max)) handler(event)
    }

    /** Any command, dispatched exactly as its own method. */
    @Throws(EmbedException::class)
    public fun execute(command: Command): CommandOutcome = handle.execute(command)

    @Throws(EmbedException::class)
    public fun status(): EmbedStatus = handle.status()

    /**
     * Signals the core to stop and returns at once, keeping the handle: the
     * queue then yields one `Closed`, and every command is refused. [close]
     * still frees it.
     */
    public fun shutdown() {
        if (!closed.get()) handle.shutdown()
    }

    /** Stops the core, waits at most 1 s for its thread, and frees the handle. Idempotent. */
    override fun close() {
        closeAndWait(CLOSE_WAIT_MS)
    }

    /**
     * [close] with its own bound: true when the core's thread joined in time,
     * so a host may unload the library. A false still frees the handle.
     */
    public fun closeAndWait(timeoutMs: Long): Boolean = release { handle.shutdownAndWait(timeoutMs) }

    /** Stops and frees without waiting: for a thread that must not block. */
    public fun closeNow() {
        release {
            handle.shutdown()
            true
        }
    }

    private inline fun release(stop: () -> Boolean): Boolean {
        if (!closed.compareAndSet(false, true)) return true
        fetch?.shutdown()
        val joined = stop()
        handle.close()
        return joined
    }

    private fun intercepted(event: EmbedEvent): Boolean {
        val fetch = fetch ?: return false
        if (event !is EmbedEvent.TokenNeeded) return false
        fetch.submit(event.reason)
        return true
    }

    public companion object {
        /** The default per-frame cap, as in every other engine kit. */
        public const val DEFAULT_DRAIN: Int = 256

        /** This kit's version: the trio's one VERSION. */
        public const val VERSION: String = BuildVersion.VERSION

        private const val CLOSE_WAIT_MS = 1_000L

        /**
         * Loads the natives, then starts the core. A [tokenFetcher] answers
         * the poll source's `TokenNeeded`, so it cannot be set beside an
         * Http token source.
         *
         * @throws LingaraNativesMissingException when no usable native library is present.
         * @throws EmbedException.Failed when the core refuses the config.
         */
        @JvmStatic
        @JvmOverloads
        @Throws(EmbedException::class)
        public fun start(
            config: EmbedConfig = EmbedConfig(),
            tokenFetcher: TokenFetcher? = null,
        ): LingaraEmbed = startAs(KitSurface.KOTLIN, config, tokenFetcher)

        /** [start] for a surface built on this one: the Java facade names itself instead. */
        @JvmStatic
        @Throws(EmbedException::class)
        public fun startAs(
            surface: KitSurface,
            config: EmbedConfig,
            tokenFetcher: TokenFetcher?,
        ): LingaraEmbed {
            require(tokenFetcher == null || config.tokenSource !is TokenSourceConfig.Http) {
                "a TokenFetcher answers the poll token source; it cannot be set with an Http token source"
            }
            NativeLoader.ensureLoaded()
            val named = config.copy(userAgentSuffix = userAgentSuffix(surface, config.userAgentSuffix))
            return LingaraEmbed(EmbedHandle.start(named), tokenFetcher)
        }
    }
}

/** Which JVM surface a handle was started through: its User-Agent token. */
public enum class KitSurface(
    internal val artefact: String,
) {
    KOTLIN("lingara-embed-kotlin"),
    JAVA("lingara-embed-java"),
}

/**
 * The kit's own token ahead of the game's suffix (ADR 1.10.26ae D3), so
 * Lingara can tell the two JVM surfaces apart. The core puts the library's
 * token and `lingara-embed/<version>` before it.
 */
internal fun userAgentSuffix(
    surface: KitSurface,
    game: String?,
): String {
    val kit = "${surface.artefact}/${BuildVersion.VERSION}"
    return if (game.isNullOrBlank()) kit else "$kit $game"
}
