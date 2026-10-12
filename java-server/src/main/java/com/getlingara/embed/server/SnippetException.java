package com.getlingara.embed.server;

/**
 * A widget-snippet option that breaks its rule (ADR 1.10.26af D7, ES4). {@link #field()} is the
 * first refused option by its contract name: {@code planId}, {@code surface}, {@code theme}, {@code
 * locale}, {@code lingaraOrigin}, {@code title}, {@code id}, {@code tokenEndpoint}, {@code nonce}
 * or {@code elementSrc}. The message never carries the value.
 */
public final class SnippetException extends IllegalArgumentException {
  private static final long serialVersionUID = 1L;

  /** The refused option's contract name. */
  private final String field;

  SnippetException(String field) {
    super("the widget snippet's " + field + " is not valid");
    this.field = field;
  }

  /**
   * Returns the refused option's contract name.
   *
   * @return the field
   */
  public String field() {
    return field;
  }
}
