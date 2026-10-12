# frozen_string_literal: true

module Lingara
  module Embed
    # A verified event and the player it concerns (ADR 1.10.26af D8).
    # +event+ is the library's Lingara::Events::Event (UnknownEvent for a
    # type newer than the library); +player_ref+ is nil for a subject the
    # directory does not know: the integrator's own, or a deleted player.
    RoutedEvent = Data.define(:event, :player_ref)
  end
end
