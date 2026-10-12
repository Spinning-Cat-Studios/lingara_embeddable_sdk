package com.getlingara.embed.server;

import java.util.Optional;

/**
 * The integrator's store of {@code player_ref} and {@code subject} (ADR 1.10.26af D5): the one
 * mapping a kit cannot derive, because {@code subject} is a pairwise HMAC Lingara computes. The
 * mint's answer is the only moment both are in one place, so {@link EmbedServer} records it here.
 *
 * <p>Implement it over your own database, beside your player rows: two columns, {@code player_ref}
 * and {@code subject}, with a unique index on {@code subject} and a plain one on {@code
 * player_ref}. One player may own more than one subject (a guest's, and a linked account's).
 * Whatever a method throws, the kit propagates unchanged.
 */
public interface PlayerDirectory {
  /**
   * Upserts the row keyed on {@code subject}. A guest's subject is stable across mints, so
   * remembering it again changes nothing.
   *
   * @param playerRef your reference for the player
   * @param subject the mint's {@code lgr_sub_}
   */
  void remember(String playerRef, String subject);

  /**
   * Returns the player a subject belongs to, or empty for one this store never saw (your own
   * subject, a deleted player).
   *
   * @param subject an event's subject
   * @return the player, if known
   */
  Optional<String> playerRefFor(String subject);

  /**
   * Removes every row for the player. Idempotent.
   *
   * @param playerRef your reference for the player
   */
  void forget(String playerRef);
}
