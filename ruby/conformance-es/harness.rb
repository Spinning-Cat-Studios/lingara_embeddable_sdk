# frozen_string_literal: true

# The ES harness on the Ruby server kit (ADR 1.10.26af D13): every ES case
# through Lingara::Embed's public surface and nothing else, the Rust
# reference harness (core/conformance-es) ported.
#
# `conformance-server run --tier es --lang ruby -- <this>` starts it, cwd the
# repository root, with the server's URL, the results path, the receiver's
# two webhook secrets and the visibly fake client credentials. It binds its
# webhook receiver (WEBrick through Rack) to port 0, prints `listening
# <port>`, and then for each case: fetches it, builds a fresh client, server
# and directory, arms it, runs the `kit:` jobs, waits for every delivery,
# judges what only the caller sees, finishes, and writes one result line.
# There is no skip.

require "json"
require "net/http"
require "lingara/embed"
require "rackup"
require "webrick"

module EsHarness
  LANG = "ruby"
  SECRET_PREFIXES = %w[lgr_et_ lgr_cs_ lgr_whsec_].freeze

  # Every line the routers' on_error and Kernel#warn (D8's default
  # facility, and the library's DefaultLogger) wrote during a case.
  class Sink
    def initialize
      @lines = []
      @lock = Mutex.new
    end

    def push(line)
      @lock.synchronize { @lines << line.to_s }
    end

    def take
      @lock.synchronize { @lines.slice!(0..) }
    end
  end

  # Kernel#warn into the sink: Kernel#warn calls Warning.warn.
  module CapturedWarnings
    class << self
      attr_accessor :sink
    end

    def warn(message, category: nil, **)
      CapturedWarnings.sink ? CapturedWarnings.sink.push(message) : super
    end
  end

  # The case's MemoryDirectory, with its lookups counted: a lookup changes
  # no contents, so only a count shows a forged delivery never reached it.
  class Recording < Lingara::Embed::MemoryDirectory
    attr_reader :lookups

    def initialize(seed)
      super()
      @lookups = 0
      seed.each { |subject, player_ref| remember(player_ref, subject) }
    end

    def player_ref_for(subject)
      @lookups += 1
      super
    end
  end

  # What one delivery's handler saw.
  Seen = Data.define(:player_ref, :event_id, :unknown)

  # The armed case, as the receiver needs it.
  class Armed
    attr_reader :deliver, :directory, :seen, :answered

    def initialize(deliver, directory)
      @deliver = deliver
      @directory = directory
      @seen = Array.new(deliver.size)
      @received = 0
      @answered = 0
    end

    def claim
      n = @received
      @received += 1
      [n, @deliver[n] || {}]
    end

    def answer!
      @answered += 1
    end
  end

  # The webhook receiver. Delivery n of the armed case is answered through a
  # PlayerRouter built from that item's router.secrets, whose handler
  # records what it saw and raises when the item's handler is `raise`.
  class Receiver
    attr_reader :sink

    def initialize(secrets, sink)
      @secrets = secrets
      @sink = sink
      @lock = Mutex.new
      @armed = nil
    end

    def arm(armed)
      @lock.synchronize { @armed = armed }
    end

    def armed
      @lock.synchronize { @armed }
    end

    def router(item, directory)
      names = item.dig("router", "secrets") || ["primary"]
      secrets = names.map { |name| @secrets.fetch(name) { raise ArgumentError, "no secret named #{name}" } }
      Lingara::Embed::PlayerRouter.new(secrets, directory: directory, on_error: ->(e) { @sink.push("on_error: #{e.message}") })
    end

    def call(env)
      body = env["rack.input"]&.read.to_s
      armed, n, item = @lock.synchronize { @armed && [@armed, *@armed.claim] }
      return [409, {}, []] unless armed
      status = deliver(armed, n, item, body, env)
      @lock.synchronize { armed.answer! }
      [status, {}, []]
    end

    private

    def deliver(armed, n, item, body, env)
      router(item, armed.directory).handle(body, env) { |routed| record(armed, n, routed, item["handler"] == "raise") }
    rescue ArgumentError => e
      @sink.push("harness: #{e.message}")
      500
    end

    def record(armed, n, routed, raise_it)
      seen = Seen.new(player_ref: routed.player_ref, event_id: routed.event.id,
        unknown: routed.event.is_a?(Lingara::Events::UnknownEvent))
      @lock.synchronize { armed.seen[n] = seen }
      raise "the conformance handler raised, as the case asked" if raise_it
    end
  end

  # A case's `kit:` jobs through Server, and what they left behind.
  class Jobs
    attr_reader :minted, :handoff, :error, :render

    def initialize(server)
      @server = server
      @render = false
    end

    # Every job in order; a raised error is recorded and the next job runs.
    def run(jobs)
      jobs.each do |job|
        one(job)
      rescue => e
        @error = raised(e)
      end
      self
    end

    private

    def one(job)
      if (m = job["mint"])
        @minted = @server.mint_token(m["player_ref"], scopes: m["scopes"], origin: m["origin"])
      elsif (d = job["delete"])
        @server.delete_player(d["player_ref"])
      elsif job.key?("handoff")
        @handoff = @minted && Lingara::Embed.token_handoff(@minted)
      else
        @render = true
      end
    end

    # {status, code}: an API error's, an OAuth error's `error`, `maintenance`.
    def raised(e)
      case e
      when Lingara::ApiError then {"status" => e.status, "code" => e.code}
      when Lingara::OAuthError then {"status" => e.status, "code" => e.error}
      when Lingara::MaintenanceError then {"status" => 503, "code" => "maintenance"}
      when Lingara::TransportError then {"status" => 0, "code" => e.kind.to_s}
      else {"status" => 0, "code" => e.class.name}
      end
    end
  end

  # The comparisons only the kit's caller can make (ES CONTRACT.md, What a
  # harness reports), each JSON-equal after dropping nulls.
  module Judge
    KEYS = %w[minted handoff error directory directory_lookups].freeze

    module_function

    def without_nulls(value)
      case value
      when Hash then value.reject { |_, v| v.nil? }.transform_values { |v| without_nulls(v) }
      when Array then value.map { |v| without_nulls(v) }
      else value
      end
    end

    # The six fields in wire form, the token read through its accessor and
    # expires_at as the handoff writes it.
    def wire(minted)
      {"token" => minted.token.expose_secret, "expires_at" => JSON.parse(Lingara::Embed.token_handoff(minted))["expires_at"],
       "expires_in" => minted.expires_in, "subject" => minted.subject, "scopes" => minted.scopes,
       "account_linked" => minted.account_linked}
    end

    def got(key, jobs, directory)
      case key
      when "minted" then jobs.minted && wire(jobs.minted)
      when "handoff" then jobs.handoff
      when "error" then jobs.error
      when "directory" then directory.snapshot
      else directory.lookups
      end
    end

    def kit(id, expect, jobs, directory)
      KEYS.filter_map do |key|
        want = expect.key?(key) ? without_nulls(expect[key]) : nil
        have = without_nulls(got(key, jobs, directory))
        # A key the case names must match; an error it did not expect is a
        # mismatch too.
        differs = expect.key?(key) ? have != want : (key == "error" && !have.nil?)
        "#{id}: fail #{key}: got #{have.nil? ? "nothing" : JSON.generate(have)}, expected #{JSON.generate(want)}" if differs
      end
    end

    def handler_ok?(want, got)
      return got.nil? if want == "not_called"
      return false unless want.is_a?(Hash) && got
      (!want.key?("player_ref") || want["player_ref"] == got.player_ref) &&
        (!want.key?("event_id") || want["event_id"] == got.event_id) &&
        (!want.key?("unknown") || want["unknown"] == got.unknown)
    end

    def handlers(id, deliver, seen)
      deliver.each_with_index.filter_map do |item, n|
        want = item.dig("expect", "handler")
        next if handler_ok?(want, seen[n])
        "#{id}: fail handler: deliver[#{n}] saw #{seen[n].inspect}, expected #{JSON.generate(want)}"
      end
    end
  end

  # One harness run: the receiver, then every case in order.
  class Run
    def initialize
      @base = ENV.fetch("LINGARA_EMBED_SERVER_URL")
      @sink = Sink.new
      CapturedWarnings.sink = @sink
      Warning.singleton_class.prepend(CapturedWarnings)
      primary, secondary = ENV.fetch("LINGARA_EMBED_WEBHOOK_SECRETS").split(",")
      @receiver = Receiver.new({"primary" => primary, "secondary" => secondary}.compact, @sink)
    end

    def listen
      server = WEBrick::HTTPServer.new(BindAddress: "127.0.0.1", Port: 0,
        Logger: WEBrick::Log.new($stderr, WEBrick::Log::WARN), AccessLog: [])
      server.mount("/", Rackup::Handler::WEBrick, @receiver)
      # The conformance server waits for this as the first line of stdout.
      $stdout.puts "listening #{server.config[:Port]}"
      $stdout.flush
      Thread.new { server.start }
    end

    def all
      dir = ENV.fetch("LINGARA_EMBED_CASES", "conformance/es/cases")
      ids = Dir[File.join(dir, "*", "*.yaml")].map { |f| "#{File.basename(File.dirname(f))}.#{File.basename(f, ".yaml")}" }
      File.open(ENV.fetch("LINGARA_EMBED_RESULTS"), "w") do |out|
        ids.sort.each { |id| out.puts(JSON.generate(one(id))) }
      end
    end

    private

    def http(verb, id, body = nil)
      uri = URI("#{@base}/__conformance/#{verb}/#{id}")
      response = body ? Net::HTTP.post(uri, JSON.generate(body), "Content-Type" => "application/json") : Net::HTTP.get_response(uri)
      raise "#{verb}: HTTP #{response.code}" unless response.is_a?(Net::HTTPSuccess)
      response.body.to_s.empty? ? {} : JSON.parse(response.body)
    end

    def one(id)
      kase = http("cases", id)
      mismatches = run_case(id, kase)
      {"case" => id, "lang" => LANG, "result" => mismatches.empty? ? "pass" : "fail", "mismatches" => mismatches}
    rescue => e
      {"case" => id, "lang" => LANG, "result" => "fail", "mismatches" => ["#{id}: harness: #{e.message}"]}
    ensure
      @receiver.arm(nil)
    end

    def run_case(id, kase)
      directory = Recording.new(kase.dig("directory", "seed") || {})
      deliver = kase["deliver"] || []
      @receiver.arm(Armed.new(deliver, directory))
      @sink.take
      server = Lingara::Embed::Server.new(client, directory: directory)
      mismatches = []
      begin
        http("arm", id, {})
      rescue => e
        mismatches << "cannot arm: #{e.message}"
      end
      jobs = Jobs.new(server).run(kase["kit"] || [])
      seen = wait_for(deliver.size)
      mismatches.concat(Judge.kit(id, kase["expect"] || {}, jobs, directory), Judge.handlers(id, deliver, seen))
      mismatches.concat(render(id, server, directory, jobs)) if jobs.render
      handlers = seen.map { |s| s&.player_ref }
      mismatches.concat(http("finish", id, {"handlers" => handlers}).fetch("mismatches", []))
    end

    def client
      Lingara::Client.new(client_id: ENV.fetch("LINGARA_EMBED_CLIENT_ID"), client_secret: ENV.fetch("LINGARA_EMBED_CLIENT_SECRET"),
        base_url: @base, token_url: "#{@base}/oauth/token")
    end

    # Until every delivery has been answered, or 15 s; then what each saw.
    def wait_for(count)
      deadline = Time.now + 15
      sleep 0.02 until (@receiver.armed&.answered.to_i >= count) || Time.now > deadline
      @receiver.armed&.seen&.dup || []
    end

    # ES5: no rendering of a kit object, and no captured line, carries a
    # secret prefix.
    def render(id, server, directory, jobs)
      router = @receiver.router({"router" => {"secrets" => %w[primary secondary]}}, directory)
      renderings = [server, router, directory, jobs.minted].flat_map { |o| [o.inspect, o.to_s] }
      renderings.concat([jobs.minted.token.inspect, jobs.minted.token.to_s]) if jobs.minted
      renderings.concat(@sink.take)
      leaks = renderings.count { |r| SECRET_PREFIXES.any? { |p| r.include?(p) } }
      leaks.zero? ? [] : ["#{id}: fail render: #{leaks} rendering(s) carry a secret prefix"]
    end
  end
end

harness = EsHarness::Run.new
harness.listen
harness.all
