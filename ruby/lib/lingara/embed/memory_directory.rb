# frozen_string_literal: true

module Lingara
  module Embed
    # A process-local player directory, for tests and single-process demos.
    # It is lost on restart and not shared between instances: production
    # code passes any object answering the same three methods over its own
    # database, a table of (player_ref, subject) unique on subject.
    #
    # `subject` is a pairwise HMAC computed by Lingara, so the mint's answer
    # is the only moment a player_ref and its subject are in one place
    # (ADR 1.10.26af D5). One player may own several subjects (a guest's,
    # then a linked account's), so the store is keyed on subject only.
    class MemoryDirectory
      def initialize
        @rows = {}
        @lock = Mutex.new
      end

      # Upserts the row keyed on +subject+. A guest's subject is stable
      # across mints, so remembering it again changes nothing.
      def remember(player_ref, subject)
        @lock.synchronize { @rows[subject] = player_ref }
        nil
      end

      # The player a subject belongs to, or nil.
      def player_ref_for(subject)
        @lock.synchronize { @rows[subject] }
      end

      # Removes every row for the player. Idempotent.
      def forget(player_ref)
        @lock.synchronize { @rows.delete_if { |_, owner| owner == player_ref } }
        nil
      end

      # Every row, subject => player_ref, as a copy.
      def snapshot
        @lock.synchronize { @rows.dup }
      end

      def inspect
        "#<Lingara::Embed::MemoryDirectory rows=#{@lock.synchronize { @rows.size }}>"
      end

      def to_s
        inspect
      end
    end
  end
end
