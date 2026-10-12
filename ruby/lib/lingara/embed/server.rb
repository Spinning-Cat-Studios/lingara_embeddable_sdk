# frozen_string_literal: true

module Lingara
  module Embed
    # Mint and delete (ADR 1.10.26af D4, D9): one library call each, the
    # directory written only after a success. There is no cache: every
    # #mint_token is one POST, so a token #delete_player revoked on another
    # instance is never handed out again.
    #
    #   client = Lingara::Client.new(client_id: ENV.fetch("LINGARA_CLIENT_ID"),
    #                                client_secret: ENV.fetch("LINGARA_CLIENT_SECRET"))
    #   server = Lingara::Embed::Server.new(client, directory: players)
    #
    # +client+ is a Lingara::Client holding your client credentials: its
    # token cache, retries and deprecation hook are the library's. A failed
    # call raises the library's Lingara::Error unchanged.
    class Server
      # +directory+, optional, is any object answering remember,
      # player_ref_for and forget (MemoryDirectory's shape).
      def initialize(client, directory: nil)
        @client = client
        @directory = directory
      end

      # Mints a token for one player: one POST /v1/embed/tokens, then
      # directory.remember(player_ref, subject). +scopes+ and +origin+ are
      # sent only when given; +origin+ is your page's origin, one of your
      # client's embed origins (a game's server mints without one). Returns
      # the library's Lingara::MintedToken. On a failure nothing is
      # remembered.
      def mint_token(player_ref, scopes: nil, origin: nil)
        body = {player_ref: player_ref}
        body[:scopes] = scopes unless scopes.nil?
        body[:origin] = origin unless origin.nil?
        minted = @client.create_embed_token(**body).value
        @directory&.remember(player_ref, minted.subject)
        minted
      end

      # Deletes a player and revokes their tokens, then forgets them. An
      # unknown player is a success; on a failure the row is kept.
      def delete_player(player_ref)
        @client.delete_embed_player(player_ref)
        @directory&.forget(player_ref)
        nil
      end

      def inspect
        "#<Lingara::Embed::Server client=#{@client.inspect} directory=#{!@directory.nil?}>"
      end

      def to_s
        inspect
      end
    end
  end
end
