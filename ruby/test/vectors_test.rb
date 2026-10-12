# frozen_string_literal: true

require "test_helper"

class VectorsTest < Minitest::Test
  # The vectors' camelCase options as widget_snippet's keywords.
  KEYWORDS = {
    "planId" => :plan_id, "tokenEndpoint" => :token_endpoint, "surface" => :surface, "theme" => :theme,
    "locale" => :locale, "lingaraOrigin" => :lingara_origin, "title" => :title, "id" => :id, "nonce" => :nonce,
    "elementSrc" => :element_src
  }.freeze

  def snippet_answer(options)
    {"html" => Lingara::Embed.widget_snippet(**options.to_h { |k, v| [KEYWORDS.fetch(k), v] })}
  rescue Lingara::Embed::SnippetError => e
    {"refused" => e.field}
  end

  # 1.10.26af AC7: every snippet.json vector (through widget_snippet) and
  # every token-handoff.json vector (through the internal TokenHandoff.body)
  # gives its expected answer.
  def test_every_snippet_and_handoff_vector
    cases = KitTest.vectors("snippet.json").fetch("cases")
    assert_operator cases.size, :>=, 40, "the snippet vectors are all here"
    cases.each do |c|
      want = c["expect"].transform_values { |v| v.gsub("{{version}}", Lingara::Embed::VERSION) }
      assert_equal want, snippet_answer(c["options"]), c["name"]
    end

    handoffs = KitTest.vectors("token-handoff.json")
    refute_empty handoffs
    handoffs.each do |c|
      input = c.fetch("input")
      assert_equal c["body"], Lingara::Embed::TokenHandoff.body(input["token"], input["expires_at"]), c["name"]
    end
  end

  # ES6's instant rule: the library holds expires_at as a Time, written back
  # in UTC with a Z and only the fractional digits it carries.
  def test_an_instant_is_written_back_in_utc
    cases = {
      "2026-10-01T09:27:44Z" => "2026-10-01T09:27:44Z",
      "2026-10-01T11:27:44+02:00" => "2026-10-01T09:27:44Z",
      "2026-10-01T09:27:44.500Z" => "2026-10-01T09:27:44.5Z",
      "2026-10-01T09:27:44.123456789Z" => "2026-10-01T09:27:44.123456789Z"
    }
    cases.each { |given, want| assert_equal want, Lingara::Embed::TokenHandoff.rfc3339(Time.parse(given)), given }
  end
end
