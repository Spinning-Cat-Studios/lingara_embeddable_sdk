package com.getlingara.embed.java;

import com.getlingara.embed.EmbedError;
import com.getlingara.embed.EmbedException;

/**
 * The one exception the facade throws when the core refuses a call (ADR 1.10.26ae D4). It is
 * unchecked: the generated {@code EmbedException} extends {@code kotlin.Exception}, which Java
 * treats as checked, so every facade method rethrows it as this. Read {@code getError().getKind()}.
 */
public final class LingaraEmbedJavaException extends RuntimeException {
  private static final long serialVersionUID = 1L;

  private final transient EmbedError error;

  LingaraEmbedJavaException(EmbedException cause) {
    super(cause.getMessage(), cause);
    this.error = cause instanceof EmbedException.Failed failed ? failed.getError() : null;
  }

  /** The core's error: its kind, message, status, code and retryability. */
  public EmbedError getError() {
    return error;
  }
}
