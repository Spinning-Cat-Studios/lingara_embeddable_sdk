# frozen_string_literal: true

# The Lingara embed server kit (ADR 1.10.26af): the four jobs a game's or a
# site's server does for its players, over the `lingara` library. Mint a
# player's token (Server#mint_token), hand it over (token_handoff), route a
# signed webhook to its player (PlayerRouter), and delete a player
# (Server#delete_player); widget_snippet renders the <lingara-embed> markup.
#
# This file is lib/lingara/embed.rb and the gem ships no lib/lingara.rb, so
# `require "lingara"` stays the library's.

require "lingara"

require_relative "embed/version"
require_relative "embed/routed_event"
require_relative "embed/memory_directory"
require_relative "embed/server"
require_relative "embed/player_router"
require_relative "embed/widget_snippet"
require_relative "embed/token_handoff"

module Lingara
  # The server kit's namespace. Lingara::MintedToken, Lingara::Error and
  # Lingara::Events are the library's.
  module Embed
    module_function

    # The three lines to paste into a page; see WidgetSnippet.render.
    def widget_snippet(**options)
      WidgetSnippet.render(**options)
    end

    # The JSON body a token route returns; see TokenHandoff.
    def token_handoff(minted)
      TokenHandoff.from(minted)
    end
  end
end
