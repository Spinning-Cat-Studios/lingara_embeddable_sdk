package com.getlingara.embed.server;

import java.util.List;
import java.util.Objects;
import java.util.Optional;

/**
 * What a mint may send beside the player (ADR 1.10.26af D4): both optional, and sent only when
 * given. {@code origin} is your page's origin, one of your client's registered embed origins; a
 * game's server mints without one. Immutable.
 */
public final class MintOptions {
  private static final MintOptions NONE = new MintOptions(null, null);

  private final List<String> scopes;
  private final String origin;

  private MintOptions(List<String> scopes, String origin) {
    this.scopes = scopes;
    this.origin = origin;
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
    return new MintOptions(List.copyOf(scopes), origin);
  }

  /**
   * Returns these options with the web origin to bind the token to.
   *
   * @param origin your page's origin, such as {@code https://game.example}
   * @return new options
   */
  public MintOptions withOrigin(String origin) {
    return new MintOptions(scopes, Objects.requireNonNull(origin, "origin"));
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

  @Override
  public String toString() {
    return "MintOptions{scopes=" + scopes + ", origin=" + origin + "}";
  }
}
