package com.getlingara.embed.server;

/**
 * What {@link WidgetSnippet#render} renders (ADR 1.10.26af D7). {@code planId} and {@code
 * tokenEndpoint} are required; every other option is written only when set. Nothing is checked
 * until {@code render}, which names the first refused option. Immutable; build one with {@link
 * #builder}.
 */
public final class SnippetOptions {
  final String planId;
  final String tokenEndpoint;
  final String surface;
  final String theme;
  final String locale;
  final String lingaraOrigin;
  final String title;
  final String id;
  final String nonce;
  final String elementSrc;

  private SnippetOptions(Builder b) {
    this.planId = b.planId;
    this.tokenEndpoint = b.tokenEndpoint;
    this.surface = b.surface;
    this.theme = b.theme;
    this.locale = b.locale;
    this.lingaraOrigin = b.lingaraOrigin;
    this.title = b.title;
    this.id = b.id;
    this.nonce = b.nonce;
    this.elementSrc = b.elementSrc;
  }

  /**
   * Starts the options for one widget.
   *
   * @param planId the lesson plan your server made for this player, a UUID
   * @param tokenEndpoint your token route: a path ({@code /lingara/token}) or an {@code https://}
   *     URL. The element {@code POST}s to it with the page's cookies and reads {@link
   *     TokenHandoff#body}'s JSON.
   * @return a builder
   */
  public static Builder builder(String planId, String tokenEndpoint) {
    return new Builder(planId, tokenEndpoint);
  }

  /** Builds {@link SnippetOptions}; each setter replaces the option's value. */
  public static final class Builder {
    private final String planId;
    private final String tokenEndpoint;
    private String surface;
    private String theme;
    private String locale;
    private String lingaraOrigin;
    private String title;
    private String id;
    private String nonce;
    private String elementSrc;

    private Builder(String planId, String tokenEndpoint) {
      this.planId = planId;
      this.tokenEndpoint = tokenEndpoint;
    }

    /**
     * Sets the surface; default {@code practice}.
     *
     * @param surface the surface
     * @return this builder
     */
    public Builder surface(String surface) {
      this.surface = surface;
      return this;
    }

    /**
     * Sets the theme: {@code light}, {@code dark} or {@code auto}.
     *
     * @param theme the theme
     * @return this builder
     */
    public Builder theme(String theme) {
      this.theme = theme;
      return this;
    }

    /**
     * Sets the locale, such as {@code fr} or {@code zh-Hant-TW}.
     *
     * @param locale the locale
     * @return this builder
     */
    public Builder locale(String locale) {
      this.locale = locale;
      return this;
    }

    /**
     * Sets the Lingara origin the element frames, for a non-production deployment.
     *
     * @param lingaraOrigin an {@code https://} origin, or {@code http://localhost[:port]}
     * @return this builder
     */
    public Builder lingaraOrigin(String lingaraOrigin) {
      this.lingaraOrigin = lingaraOrigin;
      return this;
    }

    /**
     * Sets the frame's accessible title: 1 to 80 characters, no control character.
     *
     * @param title the title
     * @return this builder
     */
    public Builder title(String title) {
      this.title = title;
      return this;
    }

    /**
     * Sets the element's id; default {@code lingara-embed}. Two widgets on one page need two ids.
     *
     * @param id the id
     * @return this builder
     */
    public Builder id(String id) {
      this.id = id;
      return this;
    }

    /**
     * Sets your page's CSP nonce, written on both scripts. A page whose {@code script-src} allows
     * no inline script blocks the third line silently without it.
     *
     * @param nonce this request's nonce
     * @return this builder
     */
    public Builder nonce(String nonce) {
      this.nonce = nonce;
      return this;
    }

    /**
     * Sets the element's script URL; default the CDN build of this release.
     *
     * @param elementSrc an {@code https://} URL
     * @return this builder
     */
    public Builder elementSrc(String elementSrc) {
      this.elementSrc = elementSrc;
      return this;
    }

    /**
     * Returns the options.
     *
     * @return the options
     */
    public SnippetOptions build() {
      return new SnippetOptions(this);
    }
  }
}
