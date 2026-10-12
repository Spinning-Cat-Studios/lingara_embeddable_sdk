#!/usr/bin/env bash
# The floor check over a scratch tree (ADR 1.10.26y D1; ADR 1.10.26af D1).
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
check="$here/check.sh"

# The seven manifests under $1, each spelling $3 in its idiom; LIBRARY_FLOOR is $2.
write_tree() {
  local root="$1" floor="$2" f="$3"
  local ruby_f; ruby_f="$(printf '%s' "$f" | sed -E 's/^([0-9]+\.[0-9]+\.[0-9]+)-/\1.pre./')"
  mkdir -p "$root/core" "$root/typescript" "$root/go" "$root/java-server" "$root/kotlin-server" "$root/ruby" "$root/php"
  printf '%s\n' "$floor" > "$root/LIBRARY_FLOOR"
  printf '[dependencies]\ngetrandom = "0.4"\nlingara = { version = "%s", default-features = false }\n' "$f" > "$root/core/Cargo.toml"
  printf '{\n  "devDependencies": {\n    "@lingara/api": "9.9.9"\n  },\n  "peerDependencies": {\n    "@lingara/api": "^%s"\n  }\n}\n' "$f" > "$root/typescript/package.json"
  printf 'module x\n\ngo 1.23\n\nrequire github.com/Spinning-Cat-Studios/lingara_api_clients/go v%s\n' "$f" > "$root/go/go.mod"
  printf 'dependencies {\n    api("com.getlingara:lingara-java:%s")\n}\n' "$f" > "$root/java-server/build.gradle.kts"
  printf 'dependencies {\n    api("com.getlingara:lingara-kotlin:%s")\n}\n' "$f" > "$root/kotlin-server/build.gradle.kts"
  printf 'Gem::Specification.new do |s|\n  s.add_dependency "lingara", ">= %s", "< 9"\nend\n' "$ruby_f" > "$root/ruby/lingara-embed.gemspec"
  printf '{\n  "require": {\n    "php": ">=8.2",\n    "spinningcatstudios/lingara": "^%s"\n  }\n}\n' "$f" > "$root/php/composer.json"
}

# The Rust row: one floor passes; another fails, naming it; none fails.
test_the_rust_row_holds_the_floor() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  write_tree "$tmp/same" 0.2.0-alpha.1 0.2.0-alpha.1
  bash "$check" "$tmp/same" > "$tmp/same.out" || { cat "$tmp/same.out"; echo "FAIL: one floor was refused"; return 1; }

  write_tree "$tmp/drift" 0.2.0-alpha.1 0.2.0-alpha.1
  sed -i.bak 's/0.2.0-alpha.1/0.1.0-alpha.12/' "$tmp/drift/core/Cargo.toml"
  if bash "$check" "$tmp/drift" > "$tmp/drift.out"; then echo "FAIL: a drifted Cargo.toml passed"; return 1; fi
  grep -q 'core/Cargo.toml: floor 0.1.0-alpha.12, LIBRARY_FLOOR is 0.2.0-alpha.1' "$tmp/drift.out" || { cat "$tmp/drift.out"; return 1; }

  write_tree "$tmp/none" 0.2.0-alpha.1 0.2.0-alpha.1
  printf '[dependencies]\nserde = "1"\n' > "$tmp/none/core/Cargo.toml"
  if bash "$check" "$tmp/none" > "$tmp/none.out"; then echo "FAIL: a manifest without the library passed"; return 1; fi
  grep -q 'core/Cargo.toml: declares no lingara library floor' "$tmp/none.out" || { cat "$tmp/none.out"; return 1; }
}

# 1.10.26af AC5
# Seven manifests spelling one floor in their idioms pass (the gemspec's
# `.pre.` form and the npm peer, not its devDependencies twin, included);
# each one declaring another floor fails, naming itself.
test_one_floor_in_seven_manifests() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  write_tree "$tmp/same" 0.2.0-alpha.1 0.2.0-alpha.1
  bash "$check" "$tmp/same" > "$tmp/same.out" || { cat "$tmp/same.out"; echo "FAIL: seven manifests on one floor were refused"; return 1; }
  [ "$(grep -cE '^✓ [a-z-]+/[^ ]+: 0.2.0-alpha.1$' "$tmp/same.out")" = 7 ] || { cat "$tmp/same.out"; echo "FAIL: not seven rows"; return 1; }

  local file
  for file in typescript/package.json go/go.mod java-server/build.gradle.kts kotlin-server/build.gradle.kts ruby/lingara-embed.gemspec php/composer.json; do
    write_tree "$tmp/drift" 0.2.0-alpha.1 0.2.0-alpha.1
    sed -i.bak -E 's/0\.2\.0(-|\.pre\.)alpha\.1/0.3.0\1alpha.1/' "$tmp/drift/$file"
    if bash "$check" "$tmp/drift" > "$tmp/drift.out"; then echo "FAIL: a drifted $file passed"; return 1; fi
    grep -q "$file: floor 0.3.0-alpha.1, LIBRARY_FLOOR is 0.2.0-alpha.1" "$tmp/drift.out" || { cat "$tmp/drift.out"; echo "FAIL: $file"; return 1; }
    rm -rf "$tmp/drift"
  done
}

test_the_rust_row_holds_the_floor
echo "✓ check_test.sh: test_the_rust_row_holds_the_floor"
test_one_floor_in_seven_manifests
echo "✓ check_test.sh: test_one_floor_in_seven_manifests"
