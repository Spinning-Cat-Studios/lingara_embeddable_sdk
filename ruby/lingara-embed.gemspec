# frozen_string_literal: true

# The gem `lingara-embed` (ADR 1.10.26af D1, D2): the Ruby server kit. Its one
# runtime dependency is the `lingara` library, from the library floor
# (LIBRARY_FLOOR at the repository root, in RubyGems' `.pre.` spelling) up to
# the next minor. It ships lib/lingara/embed.rb and never lib/lingara.rb, so
# it cannot shadow the library on $LOAD_PATH. require_paths stays at its
# default and no bindir is set, so the publisher reads this file without
# running it.

require_relative "lib/lingara/embed/version"

Gem::Specification.new do |spec|
  spec.name = "lingara-embed"
  spec.version = Lingara::Embed::GEM_VERSION
  spec.authors = ["Spinning Cat Studios"]
  spec.summary = "The official Ruby server kit for embedding Lingara in games and websites"
  spec.description = "Mint a player's embed token, hand it over, route a signed Lingara webhook to its player, " \
    "delete a player, and render the <lingara-embed> widget snippet, over the lingara library."
  spec.homepage = "https://github.com/Spinning-Cat-Studios/lingara_embeddable_sdk"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.3"
  spec.metadata = {
    "source_code_uri" => "https://github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/tree/main/ruby",
    "rubygems_mfa_required" => "true"
  }
  spec.files = Dir[File.join(__dir__, "{lib/**/*.rb,sig/**/*.rbs}")].map { |path| path.delete_prefix("#{__dir__}/") } +
    %w[README.md LICENSE]

  spec.add_dependency "lingara", ">= 0.2.0.pre.alpha.6", "< 0.3"
end
