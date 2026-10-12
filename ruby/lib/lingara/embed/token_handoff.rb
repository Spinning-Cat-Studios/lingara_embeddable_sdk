# frozen_string_literal: true

require "json"

module Lingara
  module Embed
    # The handoff body (ADR 1.10.26af D6, ES6): exactly
    # {"token":"lgr_et_…","expires_at":"…"}, the shape every player kit and
    # the widget read. This is the one place the kit exposes the token on
    # purpose.
    module TokenHandoff
      module_function

      # The JSON a token route returns for +minted+, a Lingara::MintedToken.
      # The library holds expires_at as a Time, so it is written back as
      # RFC 3339 in UTC; expires_in and subject are not forwarded.
      def from(minted)
        body(minted.token.expose_secret, rfc3339(minted.expires_at))
      end

      # The pure body over two strings, held to token-handoff.json: compact,
      # these two keys in this order, / and non-ASCII unescaped. A String
      # expires_at is copied byte for byte. Internal: integrators call
      # Lingara::Embed.token_handoff.
      def body(token, expires_at)
        JSON.generate({"token" => token, "expires_at" => expires_at})
      end

      # +time+ in UTC with a Z, its fractional seconds only when non-zero:
      # as many digits as it holds, at most nine, trailing zeros trimmed.
      def rfc3339(time)
        return time if time.is_a?(String)
        utc = time.getutc
        fraction = (utc.nsec.zero? ? "" : ".#{format("%09d", utc.nsec).sub(/0+\z/, "")}")
        "#{utc.strftime("%Y-%m-%dT%H:%M:%S")}#{fraction}Z"
      end
    end
  end
end
