# frozen_string_literal: true

module Lingara
  module Embed
    # The header a player kit sends its build's version in (ADR 30.10.26p D1).
    EMBED_VERSION_HEADER = "Lingara-Embed-Version"
    # Rack's spelling of it in request.env.
    EMBED_VERSION_RACK = "HTTP_LINGARA_EMBED_VERSION"

    # The Lingara-Embed-Version header a player kit sends to your token
    # route, trimmed (ADR 31.10.26f D1): pass it to Server#mint_token as
    # +embed_version:+. +headers+ is what PlayerRouter#handle takes: a Rack
    # request.env (HTTP_LINGARA_EMBED_VERSION) or a Hash of names to values,
    # read case-insensitively. A missing or blank header is nil, so the mint
    # sends no key. The value is not validated: Lingara is the authority.
    def self.embed_version_from(headers)
      headers.each do |key, value|
        key = key.to_s
        next unless key.casecmp?(EMBED_VERSION_HEADER) || key == EMBED_VERSION_RACK
        value = (value.is_a?(Array) ? value.join(", ") : value.to_s).strip
        return value.empty? ? nil : value
      end
      nil
    end

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
      # directory.remember(player_ref, subject). +scopes+, +origin+ and
      # +embed_version+ are sent only when given; +origin+ is your page's
      # origin, one of your client's embed origins (a game's server mints
      # without one), and +embed_version+ is the player build's declared
      # version, read with Lingara::Embed.embed_version_from. Lingara answers
      # an outdated one with the token's update_available. Returns the
      # library's Lingara::MintedToken. On a failure nothing is remembered.
      def mint_token(player_ref, scopes: nil, origin: nil, embed_version: nil)
        body = {player_ref: player_ref}
        body[:scopes] = scopes unless scopes.nil?
        body[:origin] = origin unless origin.nil?
        body[:embed_version] = embed_version unless embed_version.nil?
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
