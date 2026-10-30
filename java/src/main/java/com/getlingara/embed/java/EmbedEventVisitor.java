package com.getlingara.embed.java;

import com.getlingara.embed.EmbedEvent;

/**
 * One method per {@link EmbedEvent} subclass, for {@link LingaraEmbedJava#visit}: Java 17 has no
 * pattern matching for {@code switch} over a sealed hierarchy, so this is its exhaustive dispatch
 * (ADR 1.10.26ae D4). {@code unknown} is a variant of a newer core.
 *
 * @param <R> what each method returns
 */
public interface EmbedEventVisitor<R> {
  R tokenNeeded(EmbedEvent.TokenNeeded event);

  R stream(EmbedEvent.Stream event);

  R lingara(EmbedEvent.Lingara event);

  R worldSent(EmbedEvent.WorldSent event);

  R practice(EmbedEvent.Practice event);

  R npcTurn(EmbedEvent.NpcTurn event);

  R checkpointReached(EmbedEvent.CheckpointReached event);

  R dialogueEnded(EmbedEvent.DialogueEnded event);

  /** A stored dialogue's revision is loaded (ADR 28.10.26i); {@code checkpointReached} follows. */
  R dialogueLoaded(EmbedEvent.DialogueLoaded event);

  /** A stored dialogue could not load; its id is gone. */
  R dialogueLoadFailed(EmbedEvent.DialogueLoadFailed event);

  R error(EmbedEvent.Error event);

  R log(EmbedEvent.Log event);

  R closed(EmbedEvent.Closed event);

  R unknown(EmbedEvent.Unknown event);
}
