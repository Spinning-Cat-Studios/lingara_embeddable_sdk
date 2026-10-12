# frozen_string_literal: true

require "test_helper"

class GemspecTest < Minitest::Test
  GEMSPEC = File.join(KitTest::RUBY_DIR, "lingara-embed.gemspec")

  # 1.10.26af AC23: the gemspec has exactly one runtime dependency, the
  # lingara library, from LIBRARY_FLOOR in RubyGems' spelling to below the
  # next minor; and the gem ships lib/lingara/embed.rb and no lib/lingara.rb,
  # so it cannot shadow the library on $LOAD_PATH.
  def test_the_only_runtime_dependency_is_the_library
    spec = Gem::Specification.load(GEMSPEC)
    assert_equal "lingara-embed", spec.name
    assert_equal Lingara::Embed::GEM_VERSION, spec.version.to_s
    assert_equal ["lingara"], spec.runtime_dependencies.map(&:name)
    floor = File.read(File.join(KitTest::ROOT, "LIBRARY_FLOOR")).strip.sub("-", ".pre.")
    major, minor = floor.split(".").map(&:to_i)
    assert_equal Gem::Requirement.new(">= #{floor}", "< #{major}.#{minor + 1}"), spec.runtime_dependencies.first.requirement
    assert_equal Gem::Requirement.new(">= 3.3"), spec.required_ruby_version
    assert_equal ["lib"], spec.require_paths
    assert_includes spec.files, "lib/lingara/embed.rb"
    refute_includes spec.files, "lib/lingara.rb"
    refute(spec.files.any? { |f| f.start_with?("test/", "conformance-es/", "vendor/") })
  end
end
