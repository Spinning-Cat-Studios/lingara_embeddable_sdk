package com.getlingara.embed

/**
 * A credential: an embed token, or a header value the Http token source
 * sends (ADR 1.10.26ae D2, D7). Every token and header value that crosses
 * into the core is one, so a [Command.ProvideToken], an [EmbedConfig] or a
 * [Header] can be logged whole: [toString] is `[REDACTED]`.
 */
public class Secret(
    private val value: String,
) {
    /** The raw value. The one accessor that does not redact. */
    public fun exposeSecret(): String = value

    override fun toString(): String = REDACTED

    override fun equals(other: Any?): Boolean = other is Secret && other.value == value

    override fun hashCode(): Int = value.hashCode()

    public companion object {
        /** What every rendering of a [Secret] writes: the core's own word. */
        public const val REDACTED: String = "[REDACTED]"
    }
}
