# frozen_string_literal: true

require "json"

module Lingara
  module Embed
    # The handoff body (ADR 1.10.26af D6, ES6): exactly
    # {"token":"lgr_et_…","expires_at":"…"}, the shape every player kit and
    # the widget read, plus "update_available" when the mint answered one
    # (ADR 31.10.26f D1). This is the one place the kit exposes the token on
    # purpose.
    module TokenHandoff
      module_function

      # The JSON a token route returns for +minted+, a Lingara::MintedToken.
      # The library holds expires_at as a Time, so it is written back as
      # RFC 3339 in UTC; expires_in and subject are not forwarded. The
      # mint's update_available, its notice that the player's declared build
      # is behind, deprecated or withdrawn, is forwarded when there is one.
      def from(minted)
        body(minted.token.expose_secret, rfc3339(minted.expires_at), minted.update_available)
      end

      # The pure body, held to token-handoff.json: compact, "token",
      # "expires_at" and, only when +update_available+ (a
      # Lingara::UpdateAvailable) is given, "update_available", in this
      # order; / and non-ASCII unescaped. Without a notice the body is
      # exactly the two-key body. A String expires_at is copied byte for
      # byte. Internal: integrators call Lingara::Embed.token_handoff.
      def body(token, expires_at, update_available = nil)
        fields = {"token" => token, "expires_at" => expires_at}
        fields["update_available"] = notice(update_available) unless update_available.nil?
        JSON.generate(fields)
      end

      # The notice from its typed value, never a raw Hash: "latest",
      # "deprecated", "message", in this order, all three always written and
      # a nil kept as JSON null.
      def notice(update)
        {"latest" => update.latest, "deprecated" => update.deprecated, "message" => update.message}
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
