package com.getlingara.embed.server;

import com.getlingara.client.MintedToken;
import com.getlingara.client.model.UpdateAvailable;

/**
 * The handoff body (ADR 1.10.26af D6, ES6): {@code {"token":"lgr_et_…","expires_at":"…"}}, the
 * shape every player kit and the widget read, and since ADR 31.10.26f a third key, {@code
 * update_available}, when the mint answered one. This is the one place a kit exposes the token on
 * purpose.
 */
public final class TokenHandoff {
  private TokenHandoff() {}

  /**
   * Returns the JSON your token route sends to the player. {@code expires_at} is the server's
   * string, copied byte for byte; {@code expires_in} and {@code subject} are not forwarded. The
   * mint's {@code update_available} is forwarded when present.
   *
   * @param minted the library's minted token
   * @return the compact JSON body
   */
  public static String body(MintedToken minted) {
    return handoffBody(
        minted.token().exposeSecret(), minted.expiresAt(), minted.updateAvailable().orElse(null));
  }

  /**
   * The pure body, held to {@code token-handoff.json}: compact, {@code token} then {@code
   * expires_at}, then {@code update_available} only when given (its keys {@code latest}, {@code
   * deprecated}, {@code message}, nulls kept); {@code /} and non-ASCII unescaped.
   */
  static String handoffBody(String token, String expiresAt, UpdateAvailable updateAvailable) {
    String notice =
        updateAvailable == null ? "" : ",\"update_available\":" + noticeJson(updateAvailable);
    return "{\"token\":" + json(token) + ",\"expires_at\":" + json(expiresAt) + notice + "}";
  }

  /** The notice in the wire order (V1 §6), every key written, a null kept. */
  private static String noticeJson(UpdateAvailable n) {
    return "{\"latest\":"
        + jsonOrNull(n.getLatest())
        + ",\"deprecated\":"
        + Boolean.TRUE.equals(n.getDeprecated())
        + ",\"message\":"
        + jsonOrNull(n.getMessage())
        + "}";
  }

  private static String jsonOrNull(String value) {
    return value == null ? "null" : json(value);
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
