# frozen_string_literal: true

# The Ruby server kit examples the documentation site shows (ADR 1.10.26af
# D13). Each marked region is vendored at a released tag; make test-ruby
# parses this file under -w, lints it, and loads it against the kit. Plain
# Rack: an env in, a [status, headers, body] triplet out, so the same lines
# sit inside Sinatra, Roda or a Rails controller.

require "json"
require "lingara/embed"

module LingaraEmbedSnippets
  # lingara:begin webhook
  # Your own table: CREATE TABLE lingara_players (player_ref TEXT NOT NULL,
  # subject TEXT NOT NULL UNIQUE); CREATE INDEX ON lingara_players (player_ref).
  # Any object answering these three methods is a directory; this one is over
  # Sequel (`db = Sequel.connect(ENV.fetch("DATABASE_URL"))`).
  class PlayerTable
    def initialize(db)
      @rows = db[:lingara_players]
    end

    def remember(player_ref, subject)
      @rows.insert_conflict(target: :subject, update: {player_ref: player_ref})
        .insert(player_ref: player_ref, subject: subject)
    end

    def player_ref_for(subject)
      @rows.where(subject: subject).get(:player_ref)
    end

    def forget(player_ref)
      @rows.where(player_ref: player_ref).delete
    end
  end

  # POST /lingara/webhook. Pass the raw body: the signature covers its bytes.
  def self.webhook(env, router)
    status = router.handle(env["rack.input"].read, env) do |routed|
      # Delivery is at least once: skip an event.id you have already seen.
      # Answer within 10 s; enqueue anything slow and return.
      PracticeJobs.enqueue(routed.player_ref, routed.event) if routed.player_ref
    end
    [status, {}, []]
  end
  # lingara:end

  # Your own queue.
  module PracticeJobs
    def self.enqueue(player_ref, event)
    end
  end

  # lingara:begin mint
  # POST /lingara/token, behind your own sign-in. Rate-limit it per session.
  def self.token(env, server)
    player = env["warden"]&.user or return [401, {}, []]
    minted = server.mint_token(player.id.to_s, origin: "https://game.example")
    [200, {"content-type" => "application/json", "cache-control" => "no-store"}, [Lingara::Embed.token_handoff(minted)]]
  end
  # lingara:end

  # lingara:begin deletePlayer
  # From your account-deletion path: revokes the player's tokens at Lingara
  # and forgets their rows. An unknown player is a success.
  def self.delete_account(server, player)
    server.delete_player(player.id.to_s)
  end
  # lingara:end

  # lingara:begin widgetSnippet
  # The plan your server made for this player, rendered into your page.
  # nonce: your page's CSP nonce, when its script-src allows no inline script.
  def self.widget(plan_id, csp_nonce)
    Lingara::Embed.widget_snippet(plan_id: plan_id, token_endpoint: "/lingara/token", locale: "fr", nonce: csp_nonce)
  end
  # lingara:end

  # How the pieces above are built once, at startup.
  def self.build(db)
    client = Lingara::Client.new(client_id: ENV.fetch("LINGARA_CLIENT_ID"), client_secret: ENV.fetch("LINGARA_CLIENT_SECRET"))
    players = PlayerTable.new(db)
    server = Lingara::Embed::Server.new(client, directory: players)
    router = Lingara::Embed::PlayerRouter.new(ENV.fetch("LINGARA_WEBHOOK_SECRET"), directory: players)
    [server, router]
  end
end
