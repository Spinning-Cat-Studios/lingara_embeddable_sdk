package com.getlingara.embed.server.kotlin.conformance

import com.getlingara.embed.server.kotlin.MemoryPlayerDirectory
import com.getlingara.embed.server.kotlin.PlayerDirectory
import java.util.concurrent.atomic.AtomicInteger
import java.util.logging.Handler
import java.util.logging.Level
import java.util.logging.LogRecord
import java.util.logging.Logger

/**
 * The case's [MemoryPlayerDirectory] with its `playerRefFor` calls counted: a lookup changes no
 * contents, so only a count shows a forged delivery never reached the store.
 */
class Recording : PlayerDirectory {
    private val inner = MemoryPlayerDirectory()
    private val lookups = AtomicInteger()

    fun lookups(): Int = lookups.get()

    fun snapshot(): Map<String, String> = inner.snapshot()

    override suspend fun remember(
        playerRef: String,
        subject: String,
    ) = inner.remember(playerRef, subject)

    override suspend fun playerRefFor(subject: String): String? {
        lookups.incrementAndGet()
        return inner.playerRefFor(subject)
    }

    override suspend fun forget(playerRef: String) = inner.forget(playerRef)

    override fun toString(): String = "Recording($inner)"
}

/** Every line the routers' `onError` and `System.Logger` wrote. */
class Sink {
    private val lines = mutableListOf<String>()

    fun push(line: String) {
        synchronized(lines) { lines.add(line) }
    }

    fun take(): List<String> = synchronized(lines) { lines.toList().also { lines.clear() } }

    /**
     * Routes the root java.util.logging logger, where `System.getLogger` lands by default, into the
     * sink: the router's default `onError` facility.
     */
    fun captureLogs() {
        val root = Logger.getLogger("")
        root.handlers.forEach(root::removeHandler)
        root.level = Level.ALL
        root.addHandler(
            object : Handler() {
                override fun publish(r: LogRecord) = push("${r.level} ${r.loggerName}: ${r.message} ${r.thrown ?: ""}")

                override fun flush() = Unit

                override fun close() = Unit
            },
        )
    }
}
