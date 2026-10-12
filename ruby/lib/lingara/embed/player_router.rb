# frozen_string_literal: true

module Lingara
  module Embed
    # Routes a Lingara webhook to its player (ADR 1.10.26af D8): the
    # library's verifier on the raw body first, then the directory, so a
    # forged event never reaches your store.
    #
    #   router = Lingara::Embed::PlayerRouter.new(ENV.fetch("LINGARA_WEBHOOK_SECRET"), directory: players)
    #   status = router.handle(request.body.read, request.env) { |routed| enqueue(routed) }
    #
    # +secrets+ is one lgr_whsec_ secret or, during a rotation, a list; a
    # malformed one raises the library's ArgumentError here, at startup.
    # +directory+ is any object answering remember, player_ref_for and
    # forget. +on_error+ receives what a 500 swallowed; its default writes
    # one line through Kernel#warn.
    class PlayerRouter
      DEFAULT_ON_ERROR = ->(error) { Kernel.warn("lingara webhook: #{error.class}: #{error.message}") }

      def initialize(secrets, directory:, on_error: DEFAULT_ON_ERROR)
        @webhook = Lingara::Events::Webhook.new(secrets)
        @directory = directory
        @on_error = on_error
      end

      # Verifies the raw +body+ (a String, exactly as received) against
      # +headers+ (a Hash or a Rack env), then looks the subject up. Raises
      # Lingara::Events::VerificationError before the directory is asked.
      def route(body, headers)
        event = @webhook.verify(body, headers)
        RoutedEvent.new(event: event, player_ref: @directory.player_ref_for(event.subject))
      end

      # #route, then the block, answered as the status your route sends with
      # an empty body: 204 handled, 401 a bad signature, 400 a malformed
      # payload under a valid signature, 500 the directory or the block
      # raised (the error goes to on_error only). Every non-2xx is
      # redelivered, so a slow block should enqueue and return.
      def handle(body, headers, &on_event)
        raise ArgumentError, "handle needs a block, the event handler" unless on_event
        dispatch(body, headers, on_event)
      end

      def inspect
        "#<Lingara::Embed::PlayerRouter webhook=#{@webhook.inspect}>"
      end

      def to_s
        inspect
      end

      private

      def dispatch(body, headers, on_event)
        routed = verified_route(body, headers)
        return routed if routed.is_a?(Integer)
        on_event.call(routed)
        204
      rescue => e
        @on_error.call(e)
        500
      end

      # The routed event, or the status a verification failure answers.
      def verified_route(body, headers)
        route(body, headers)
      rescue Lingara::Events::VerificationError => e
        (e.reason == :malformed_payload) ? 400 : 401
      end
    end
  end
end
