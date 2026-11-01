package com.getlingara.embed.server;

import java.util.List;
import java.util.Objects;
import java.util.Optional;

/**
 * What a mint may send beside the player (ADR 1.10.26af D4, 31.10.26f D1): each optional, and sent
 * only when given. {@code origin} is your page's origin, one of your client's registered embed
 * origins; a game's server mints without one. {@code embedVersion} is the player build's version,
 * read with {@link EmbedServer#embedVersionFrom}. Immutable.
 */
public final class MintOptions {
  private static final MintOptions NONE = new MintOptions(null, null, null);

  private final List<String> scopes;
  private final String origin;
  private final String embedVersion;

  private MintOptions(List<String> scopes, String origin, String embedVersion) {
    this.scopes = scopes;
    this.origin = origin;
    this.embedVersion = embedVersion;
  }

  /**
   * Returns options sending neither key: the token gets every handable scope the client holds.
   *
   * @return the empty options
   */
  public static MintOptions none() {
    return NONE;
  }

  /**
   * Returns these options with the scopes to grant, a subset of {@code events:read}, {@code
   * events:write}, {@code lesson_plans:read}, {@code lesson_plans:write} and {@code embed:play}.
   *
   * @param scopes the scopes, in the order to send them
   * @return new options
   */
  public MintOptions withScopes(List<String> scopes) {
    return new MintOptions(List.copyOf(scopes), origin, embedVersion);
  }

  /**
   * Returns these options with the web origin to bind the token to.
   *
   * @param origin your page's origin, such as {@code https://game.example}
   * @return new options
   */
  public MintOptions withOrigin(String origin) {
    return new MintOptions(scopes, Objects.requireNonNull(origin, "origin"), embedVersion);
  }

  /**
   * Returns these options with the player build's declared version, sent as {@code embed_version}.
   * It is not validated here: Lingara answers a bad one with {@code 400 invalid_request_body} or
   * {@code 400 embed_version_unknown}, and a withdrawn one with {@code 409
   * embed_version_withdrawn}. A null (what {@link EmbedServer#embedVersionFrom} returns for a
   * missing or blank header) sends no key.
   *
   * @param embedVersion the version, or null for none
   * @return new options
   */
  public MintOptions withEmbedVersion(String embedVersion) {
    return new MintOptions(scopes, origin, embedVersion);
  }

  /**
   * Returns the scopes, when given.
   *
   * @return the scopes
   */
  public Optional<List<String>> scopes() {
    return Optional.ofNullable(scopes);
  }

  /**
   * Returns the origin, when given.
   *
   * @return the origin
   */
  public Optional<String> origin() {
    return Optional.ofNullable(origin);
  }

  /**
   * Returns the declared embed version, when given.
   *
   * @return the version
   */
  public Optional<String> embedVersion() {
    return Optional.ofNullable(embedVersion);
  }

  @Override
  public String toString() {
    return "MintOptions{scopes="
        + scopes
        + ", origin="
        + origin
        + ", embedVersion="
        + embedVersion
        + "}";
  }
}
