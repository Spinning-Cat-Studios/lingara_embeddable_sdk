# frozen_string_literal: true

require "json"
require "minitest/autorun"
require "lingara/embed"

# What the kit's tests share: the repository's paths and its vectors, read
# from the repository root as every server kit reads them.
module KitTest
  RUBY_DIR = File.expand_path("..", __dir__)
  ROOT = File.expand_path("..", RUBY_DIR)

  module_function

  def vectors(name)
    JSON.parse(File.read(File.join(ROOT, "conformance/vectors", name)))
  end
end
