# frozen_string_literal: true

require "openssl"
require "test_helper"

class PlayerRouterTest < Minitest::Test
  # A MemoryDirectory that counts lookups, and fails them when asked to.
  class Counting < Lingara::Embed::MemoryDirectory
    attr_reader :lookups
    attr_accessor :fail

    def initialize
      super
      @lookups = 0
    end

    def player_ref_for(subject)
      @lookups += 1
      raise "the store is down" if fail
      super
    end
  end

  # E7's shared vector, re-signed now with its own secret so the router's
  # default clock accepts it (the router takes no clock: its verifier is the
  # library's, built from the secrets alone).
  VECTOR = KitTest.vectors("webhook-signatures.json").fetch("vectors").find { |v| v["name"] == "valid-single-signature" }
  SECRET = VECTOR.fetch("secrets").first
  BODY = VECTOR.fetch("body")

  # A Rack env carrying the Standard Webhooks headers over +body+, signed
  # now; the body itself is passed to handle as the raw String.
  def rack_env(body, signed: body)
    id = JSON.parse(BODY).fetch("id")
    timestamp = Time.now.to_i.to_s
    key = SECRET.delete_prefix("lgr_whsec_").unpack1("m0")
    mac = OpenSSL::HMAC.digest("SHA256", key, "#{id}.#{timestamp}.#{signed}")
    {"REQUEST_METHOD" => "POST", "CONTENT_TYPE" => "application/json", "HTTP_WEBHOOK_ID" => id,
     "HTTP_WEBHOOK_TIMESTAMP" => timestamp, "HTTP_WEBHOOK_SIGNATURE" => "v1,#{[mac].pack("m0")}"}
  end

  def router(fail: false)
    directory = Counting.new
    directory.remember("p1", "lgr_sub_conformance")
    directory.fail = fail
    errors = []
    [Lingara::Embed::PlayerRouter.new(SECRET, directory: directory, on_error: ->(e) { errors << e.message }), directory, errors]
  end

  # 1.10.26af AC15: through handle, with a Rack env as the headers, a
  # tampered body is 401 and never reaches the directory or the block; a
  # valid event reaches the block with the remembered player_ref; an unknown
  # subject arrives with none; a raising handler and a raising directory are
  # each 500 and reach on_error; a valid signature over a malformed payload
  # is 400.
  def test_handle_verifies_before_resolving
    seen = []
    record = ->(routed) { seen << routed.player_ref }

    r, dir, = router
    tampered = BODY.sub("At the night market", "At the night markets")
    assert_equal 401, r.handle(tampered, rack_env(BODY, signed: BODY), &record)
    assert_equal [0, 0], [dir.lookups, seen.size]

    assert_equal 204, r.handle(BODY, rack_env(BODY), &record)
    assert_equal 1, dir.lookups
    assert_equal ["p1"], seen.pop(1)

    dir.forget("p1")
    assert_equal 204, r.handle(BODY, rack_env(BODY), &record)
    assert_equal [nil], seen.pop(1), "an unknown subject routes to no player"

    r, _, errors = router
    assert_equal(500, r.handle(BODY, rack_env(BODY)) { raise "the handler broke" })
    assert_equal ["the handler broke"], errors

    r, _, errors = router(fail: true)
    assert_equal 500, r.handle(BODY, rack_env(BODY), &record)
    assert_equal ["the store is down"], errors
    assert_empty seen, "a failed lookup never calls the block"

    r, = router
    assert_equal 400, r.handle("not json", rack_env("not json"), &record), "a valid signature over a malformed payload"
  end

  # D8's default on_error is Kernel#warn, and a rendering of the router
  # carries no secret.
  def test_the_default_on_error_warns_and_nothing_renders_a_secret
    r = Lingara::Embed::PlayerRouter.new(SECRET, directory: Lingara::Embed::MemoryDirectory.new)
    _, err = capture_io { assert_equal(500, r.handle(BODY, rack_env(BODY)) { raise "boom" }) }
    assert_match(/lingara webhook: RuntimeError: boom/, err)
    refute_includes r.inspect, "lgr_whsec_"
  end
end
