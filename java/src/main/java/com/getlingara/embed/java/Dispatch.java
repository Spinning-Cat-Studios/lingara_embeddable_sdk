package com.getlingara.embed.java;

import com.getlingara.embed.EmbedEvent;

/**
 * {@link LingaraEmbedJava#visit}'s dispatch, in two halves to stay within the complexity budget.
 */
final class Dispatch {
  private Dispatch() {}

  static <R> R visit(EmbedEvent event, EmbedEventVisitor<R> visitor) {
    if (event instanceof EmbedEvent.TokenNeeded e) {
      return visitor.tokenNeeded(e);
    } else if (event instanceof EmbedEvent.Stream e) {
      return visitor.stream(e);
    } else if (event instanceof EmbedEvent.Lingara e) {
      return visitor.lingara(e);
    } else if (event instanceof EmbedEvent.WorldSent e) {
      return visitor.worldSent(e);
    } else if (event instanceof EmbedEvent.Practice e) {
      return visitor.practice(e);
    }
    return rest(event, visitor);
  }

  private static <R> R rest(EmbedEvent event, EmbedEventVisitor<R> visitor) {
    if (event instanceof EmbedEvent.NpcTurn e) {
      return visitor.npcTurn(e);
    } else if (event instanceof EmbedEvent.CheckpointReached e) {
      return visitor.checkpointReached(e);
    } else if (event instanceof EmbedEvent.DialogueEnded e) {
      return visitor.dialogueEnded(e);
    } else if (event instanceof EmbedEvent.DialogueLoaded e) {
      return visitor.dialogueLoaded(e);
    } else if (event instanceof EmbedEvent.DialogueLoadFailed e) {
      return visitor.dialogueLoadFailed(e);
    }
    return last(event, visitor);
  }

  private static <R> R last(EmbedEvent event, EmbedEventVisitor<R> visitor) {
    if (event instanceof EmbedEvent.Error e) {
      return visitor.error(e);
    } else if (event instanceof EmbedEvent.Log e) {
      return visitor.log(e);
    } else if (event instanceof EmbedEvent.Closed e) {
      return visitor.closed(e);
    } else if (event instanceof EmbedEvent.Unknown e) {
      return visitor.unknown(e);
    }
    throw new IllegalArgumentException("an EmbedEvent with no visitor method: " + event);
  }
}
