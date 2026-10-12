package com.getlingara.embed.server;

import java.nio.charset.StandardCharsets;
import java.util.regex.Pattern;

/**
 * ES4's option rules (ADR 1.10.26af D7), one method per row of the contract's table. Every pattern
 * is ASCII-only and matched against the whole value.
 */
final class SnippetRules {
  private static final Pattern PLAN_ID =
      Pattern.compile("[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}");
  private static final Pattern SURFACE = Pattern.compile("[a-z][a-z0-9-]{0,31}");
  private static final Pattern THEME = Pattern.compile("light|dark|auto");
  private static final Pattern LOCALE = Pattern.compile("[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}");
  private static final Pattern ORIGIN =
      Pattern.compile(
          "(https://[A-Za-z0-9-]+(\\.[A-Za-z0-9-]+)*(:[0-9]{1,5})?"
              + "|http://(localhost|127\\.0\\.0\\.1)(:[0-9]{1,5})?)/?");
  private static final Pattern ID = Pattern.compile("[A-Za-z][A-Za-z0-9_-]{0,63}");
  private static final Pattern NONCE = Pattern.compile("[A-Za-z0-9+/=_-]{8,128}");
  private static final int MAX_TITLE = 80;
  private static final int MAX_URL_BYTES = 2048;

  private SnippetRules() {}

  /**
   * Returns the first refused option's name, in the contract's order, or null.
   *
   * @param o the options
   * @param surface the surface, defaulted
   * @param id the id, defaulted
   */
  static String firstRefused(SnippetOptions o, String surface, String id) {
    Object[][] rules = {
      {"planId", matches(PLAN_ID, o.planId)},
      {"surface", matches(SURFACE, surface)},
      {"theme", o.theme == null || matches(THEME, o.theme)},
      {"locale", o.locale == null || matches(LOCALE, o.locale)},
      {"lingaraOrigin", o.lingaraOrigin == null || matches(ORIGIN, o.lingaraOrigin)},
      {"title", o.title == null || title(o.title)},
      {"id", matches(ID, id)},
      {"tokenEndpoint", tokenEndpoint(o.tokenEndpoint)},
      {"nonce", o.nonce == null || matches(NONCE, o.nonce)},
      {"elementSrc", o.elementSrc == null || elementSrc(o.elementSrc)},
    };
    for (Object[] rule : rules) {
      if (!(Boolean) rule[1]) {
        return (String) rule[0];
      }
    }
    return null;
  }

  private static boolean matches(Pattern pattern, String value) {
    return value != null && pattern.matcher(value).matches();
  }

  /** 1 to 80 scalar values, none of them a control character. */
  private static boolean title(String s) {
    int count = s.codePointCount(0, s.length());
    return count >= 1 && count <= MAX_TITLE && s.codePoints().noneMatch(SnippetRules::isControl);
  }

  /** A path that starts {@code /} but not {@code //}, or an absolute {@code https://} URL. */
  private static boolean tokenEndpoint(String s) {
    if (s == null) {
      return false;
    }
    boolean shape = (s.startsWith("/") && !s.startsWith("//")) || s.startsWith("https://");
    return shape && urlText(s);
  }

  private static boolean elementSrc(String s) {
    return s.startsWith("https://") && urlText(s);
  }

  /** At most 2048 UTF-8 bytes, with no control character or whitespace. */
  private static boolean urlText(String s) {
    return s.getBytes(StandardCharsets.UTF_8).length <= MAX_URL_BYTES
        && s.codePoints().noneMatch(c -> isControl(c) || isSpace(c));
  }

  /** Unicode category Cc. */
  static boolean isControl(int c) {
    return c <= 0x1f || (c >= 0x7f && c <= 0x9f);
  }

  /** The contract's whitespace set: U+2000 to U+200A, and these. */
  private static final String SPACES = " \u0085\u00a0\u1680\u2028\u2029\u202f\u205f\u3000";

  private static boolean isSpace(int c) {
    return (c >= 0x2000 && c <= 0x200a) || SPACES.indexOf(c) >= 0;
  }
}
