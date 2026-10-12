package com.getlingara.embed.server;

import com.getlingara.client.MintedToken;

/**
 * The handoff body (ADR 1.10.26af D6, ES6): exactly {@code {"token":"lgr_et_…","expires_at":"…"}},
 * the shape every player kit and the widget read. This is the one place a kit exposes the token on
 * purpose.
 */
public final class TokenHandoff {
  private TokenHandoff() {}

  /**
   * Returns the JSON your token route sends to the player. {@code expires_at} is the server's
   * string, copied byte for byte; {@code expires_in} and {@code subject} are not forwarded.
   *
   * @param minted the library's minted token
   * @return the compact JSON body
   */
  public static String body(MintedToken minted) {
    return handoffBody(minted.token().exposeSecret(), minted.expiresAt());
  }

  /**
   * The pure body over two strings, held to {@code token-handoff.json}: compact, these two keys in
   * this order, {@code /} and non-ASCII unescaped.
   */
  static String handoffBody(String token, String expiresAt) {
    return "{\"token\":" + json(token) + ",\"expires_at\":" + json(expiresAt) + "}";
  }

  /** A JSON string: {@code "}, {@code \} and the C0 controls escaped, nothing else. */
  private static String json(String value) {
    StringBuilder out = new StringBuilder("\"");
    for (int i = 0; i < value.length(); i++) {
      char c = value.charAt(i);
      switch (c) {
        case '"' -> out.append("\\\"");
        case '\\' -> out.append("\\\\");
        case '\n' -> out.append("\\n");
        case '\r' -> out.append("\\r");
        case '\t' -> out.append("\\t");
        case '\b' -> out.append("\\b");
        case '\f' -> out.append("\\f");
        default -> out.append(c < 0x20 ? String.format("\\u%04x", (int) c) : String.valueOf(c));
      }
    }
    return out.append('"').toString();
  }
}
