package com.getlingara.embed.server;

import com.getlingara.client.LingaraClient;
import com.getlingara.client.MintedToken;
import com.getlingara.client.model.EmbedTokenRequest;
import java.util.LinkedHashSet;
import java.util.Objects;
import java.util.Set;

/**
 * Mint and delete (ADR 1.10.26af D4, D9): one library call each, the directory written only after a
 * success. There is no cache: every {@link #mintToken} is one {@code POST}, so a token {@link
 * #deletePlayer} revoked on another instance is never handed out again.
 *
 * <p>A failed call throws the library's own {@code LingaraException} unchanged; its token cache,
 * retries and deprecation hook are the library's. Safe for concurrent use when the directory is.
 */
public final class EmbedServer {
  private final LingaraClient client;
  private final PlayerDirectory directory;

  private EmbedServer(LingaraClient client, PlayerDirectory directory) {
    this.client = Objects.requireNonNull(client, "client");
    this.directory = directory;
  }

  /**
   * A server over a client built with your client credentials, recording nothing.
   *
   * @param client the Lingara client
   * @return the server
   */
  public static EmbedServer of(LingaraClient client) {
    return new EmbedServer(client, null);
  }

  /**
   * A server that records each mint's subject in {@code directory}, and forgets a deleted player.
   *
   * @param client the Lingara client
   * @param directory your player directory
   * @return the server
   */
  public static EmbedServer of(LingaraClient client, PlayerDirectory directory) {
    return new EmbedServer(client, Objects.requireNonNull(directory, "directory"));
  }

  /**
   * Mints a token for one player with no scopes or origin named.
   *
   * @param playerRef your reference for the player: 1 to 128 bytes of UTF-8
   * @return the library's minted token
   */
  public MintedToken mintToken(String playerRef) {
    return mintToken(playerRef, MintOptions.none());
  }

  /**
   * Mints a token for one player: one {@code POST /v1/embed/tokens}, then {@code
   * remember(playerRef, subject)}. On a failure nothing is remembered.
   *
   * @param playerRef your reference for the player: 1 to 128 bytes of UTF-8
   * @param options the scopes and origin, each sent only when given
   * @return the library's minted token; hand it over with {@link TokenHandoff#body}
   * @throws IllegalArgumentException for a scope the library does not know
   */
  public MintedToken mintToken(String playerRef, MintOptions options) {
    EmbedTokenRequest request = new EmbedTokenRequest().playerRef(playerRef);
    options.scopes().ifPresent(scopes -> request.scopes(scopeSet(scopes)));
    options.origin().ifPresent(request::origin);
    MintedToken minted = client.createEmbedToken(request).body();
    if (directory != null) {
      directory.remember(playerRef, minted.subject());
    }
    return minted;
  }

  /**
   * Deletes a player and revokes their tokens, then forgets them. An unknown player is a success;
   * on a failure the row is kept. Call it from your account-deletion path.
   *
   * @param playerRef your reference for the player
   */
  public void deletePlayer(String playerRef) {
    client.deleteEmbedPlayer(playerRef);
    if (directory != null) {
      directory.forget(playerRef);
    }
  }

  private static Set<EmbedTokenRequest.ScopesEnum> scopeSet(Iterable<String> scopes) {
    Set<EmbedTokenRequest.ScopesEnum> out = new LinkedHashSet<>();
    for (String scope : scopes) {
      out.add(EmbedTokenRequest.ScopesEnum.fromValue(scope));
    }
    return out;
  }

  @Override
  public String toString() {
    return "EmbedServer{client=" + client + ", directory=" + (directory != null) + "}";
  }
}
