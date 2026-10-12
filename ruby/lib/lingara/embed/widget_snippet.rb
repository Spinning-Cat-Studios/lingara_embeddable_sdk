# frozen_string_literal: true

module Lingara
  module Embed
    # The first widget_snippet option refused. +field+ is the option's
    # contract name, camelCase (planId, tokenEndpoint, …), never its value.
    class SnippetError < ArgumentError
      attr_reader :field

      def initialize(field)
        @field = field
        super("the widget snippet's #{field} is not valid")
      end
    end

    # The widget snippet (ADR 1.10.26af D7, ES4): the <lingara-embed>
    # element and its two module scripts, rendered identically by all eight
    # server kits. The token never appears in it, only the endpoint the
    # element asks at run time.
    module WidgetSnippet
      DEFAULT_ELEMENT_SRC = "https://cdn.jsdelivr.net/npm/@lingara/embed@#{VERSION}/dist/lingara-embed.min.js"

      # Unicode category Cc.
      CONTROL = /[\u0000-\u001f\u007f-\u009f]/
      # The contract's whitespace beyond Cc.
      SPACE = /[\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]/
      ORIGIN = %r{\A(https://[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*(:[0-9]{1,5})?|http://(localhost|127\.0\.0\.1)(:[0-9]{1,5})?)/?\z}

      # Each option's rule, in the contract's order: the first refused is
      # the one raised.
      RULES = [
        [:plan_id, "planId", ->(v) { v.match?(/\A[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}\z/) }],
        [:surface, "surface", ->(v) { v.match?(/\A[a-z][a-z0-9-]{0,31}\z/) }],
        [:theme, "theme", ->(v) { %w[light dark auto].include?(v) }],
        [:locale, "locale", ->(v) { v.match?(/\A[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}\z/) }],
        [:lingara_origin, "lingaraOrigin", ->(v) { v.match?(ORIGIN) }],
        [:title, "title", ->(v) { v.length.between?(1, 80) && !v.match?(CONTROL) }],
        [:id, "id", ->(v) { v.match?(/\A[A-Za-z][A-Za-z0-9_-]{0,63}\z/) }],
        [:token_endpoint, "tokenEndpoint", ->(v) { v.match?(%r{\A(/(?!/)|https://)}) && WidgetSnippet.url_text?(v) }],
        [:nonce, "nonce", ->(v) { v.match?(%r{\A[A-Za-z0-9+/=_-]{8,128}\z}) }],
        [:element_src, "elementSrc", ->(v) { v.start_with?("https://") && WidgetSnippet.url_text?(v) }]
      ].freeze

      ATTRIBUTE_ESCAPES = {"&" => "&amp;", "<" => "&lt;", ">" => "&gt;", '"' => "&quot;", "'" => "&#39;"}.freeze
      SCRIPT_ESCAPES = {
        '"' => '\\"', "\\" => "\\\\", "<" => "\\u003c", ">" => "\\u003e", "&" => "\\u0026",
        "\u2028" => "\\u2028", "\u2029" => "\\u2029"
      }.freeze

      module_function

      # The three lines to paste into a page, with no trailing newline.
      # +plan_id+ (the plan your server made for this player, a UUID) and
      # +token_endpoint+ (your token route: a path, or an https:// URL) are
      # required; +nonce+ is your page's CSP nonce, written on both scripts.
      # Raises SnippetError naming the first option refused.
      def render(plan_id: nil, token_endpoint: nil, surface: nil, theme: nil, locale: nil,
        lingara_origin: nil, title: nil, id: nil, nonce: nil, element_src: nil)
        values = {plan_id: plan_id.to_s, token_endpoint: token_endpoint.to_s, surface: surface || "practice",
                  theme: theme, locale: locale, lingara_origin: lingara_origin, title: title, id: id || "lingara-embed",
                  nonce: nonce, element_src: element_src || DEFAULT_ELEMENT_SRC}
        check(values)
        lines(values)
      end

      # The first refused option, by its contract name, or nothing.
      def check(values)
        RULES.each do |key, field, ok|
          value = values[key]
          next if value.nil?
          raise SnippetError, field unless value.is_a?(String) && value.valid_encoding? && ok.call(value)
        end
      end

      def lines(v)
        element = +"<lingara-embed"
        [["id", v[:id]], ["surface", v[:surface]], ["plan-id", v[:plan_id]], ["theme", v[:theme]],
          ["locale", v[:locale]], ["lingara-origin", v[:lingara_origin]], ["title", v[:title]]].each do |name, value|
          element << %( #{name}="#{attribute(value)}") unless value.nil?
        end
        nonce = v[:nonce] ? %( nonce="#{attribute(v[:nonce])}") : ""
        fetch = "document.getElementById(#{script_string(v[:id])}).tokenProvider = async () => " \
          "(await fetch(#{script_string(v[:token_endpoint])}, { method: \"POST\", credentials: \"same-origin\" })).json();"
        "#{element}></lingara-embed>\n" \
          "<script type=\"module\"#{nonce} src=\"#{attribute(v[:element_src])}\"></script>\n" \
          "<script type=\"module\"#{nonce}>#{fetch}</script>"
      end

      # At most 2048 UTF-8 bytes, with no control character or whitespace.
      def url_text?(value)
        value.bytesize <= 2048 && !value.match?(CONTROL) && !value.match?(SPACE)
      end

      # An attribute value: the five characters, and nothing else.
      def attribute(value)
        value.gsub(/[&<>"']/, ATTRIBUTE_ESCAPES)
      end

      # A JSON string literal that can never close the script or open a
      # comment.
      def script_string(value)
        %("#{value.gsub(/["\\<>&\u2028\u2029]/, SCRIPT_ESCAPES)}")
      end
    end
  end
end
