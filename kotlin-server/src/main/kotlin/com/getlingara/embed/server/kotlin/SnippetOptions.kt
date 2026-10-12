package com.getlingara.embed.server.kotlin

/**
 * What [widgetSnippet] renders (ADR 1.10.26af D7). [planId] and [tokenEndpoint] are required;
 * every other option is written only when set, as in
 * `SnippetOptions(planId, "/lingara/token").apply { theme = "dark" }`. Nothing is checked until
 * [widgetSnippet], which names the first refused option.
 *
 * @property planId the lesson plan your server made for this player, a UUID
 * @property tokenEndpoint your token route: a path (`/lingara/token`) or an `https://` URL. The
 *   element `POST`s to it with the page's cookies and reads [tokenHandoff]'s JSON.
 */
public class SnippetOptions(
    public val planId: String,
    public val tokenEndpoint: String,
) {
    /** Default `practice`. */
    public var surface: String? = null

    /** `light`, `dark` or `auto`. */
    public var theme: String? = null

    /** Such as `fr` or `zh-Hant-TW`. */
    public var locale: String? = null

    /** An `https://` origin, or `http://localhost[:port]`, for a non-production deployment. */
    public var lingaraOrigin: String? = null

    /** The frame's accessible title: 1 to 80 characters, no control character. */
    public var title: String? = null

    /** Default `lingara-embed`. Two widgets on one page need two ids. */
    public var id: String? = null

    /** Your page's CSP nonce, written on both scripts; a strict page blocks the third line without it. */
    public var nonce: String? = null

    /** An `https://` URL; default the CDN build of this release. */
    public var elementSrc: String? = null
}

/**
 * A widget-snippet option that breaks its rule (ADR 1.10.26af D7, ES4). [field] is the first
 * refused option by its contract name: `planId`, `surface`, `theme`, `locale`, `lingaraOrigin`,
 * `title`, `id`, `tokenEndpoint`, `nonce` or `elementSrc`. The message never carries the value.
 */
public class SnippetException(
    public val field: String,
) : IllegalArgumentException("the widget snippet's $field is not valid")
