# typescript/ — the npm package @lingara/embed: the <lingara-embed> element
# (the browser entry) and, beside it, the `/server` subpath.
#
# The element is a frame, not a binding, so it joins no list on its own
# account: its contract is the hosted page's postMessage protocol, held by a
# copied vector file. The `/server` subpath (ADR 1.10.26af) is a server kit,
# so the directory joins ES_LANGS as `typescript`; its harness runs from the
# package root against the built dist/server. It generates nothing, so its
# codegen targets are no-ops that exist only because make/quality.mk's
# aggregates run them over $(LANGS). Every target that needs the pinned dev tools depends
# on $(TS_STAMP), a stamped `npm ci`, so a fresh checkout installs once and
# a changed lockfile reinstalls. Both test lanes depend on build-typescript:
# the tests read dist/, and a lane that passes against a stale or missing
# dist/ would grade nothing.

TS_DIR := typescript
TS_NPM := npm --prefix $(TS_DIR)
TS_BIN := $(TS_DIR)/node_modules/.bin
TS_STAMP := $(TS_DIR)/node_modules/.lingara-ci-stamp
TS_SNIPPETS := snippets/player/web
TS_SERVER_SNIPPETS := snippets/server/typescript
TS_ES_HARNESS := $(TS_DIR)/conformance-es/dist/harness.mjs

ES_LANGS += typescript
CONFORMANCE_HARNESS_es_typescript := node $(TS_ES_HARNESS)

.PHONY: lock-typescript build-typescript build-typescript-es-harness test-typescript check-typescript codegen-typescript check-codegen-typescript help-typescript

$(TS_STAMP): $(TS_DIR)/package.json $(TS_DIR)/package-lock.json
	$(TS_NPM) ci --no-audit --no-fund
	@touch $@

## Rewrite package-lock.json from package.json (after a devDependency edit).
lock-typescript:
	$(TS_NPM) install --package-lock-only --no-audit --no-fund

## dist/browser (tsc), the script-tag module and its .sri, and dist/server
## (ESM and CJS, each with its declarations).
build-typescript: $(TS_STAMP)
	$(TS_NPM) run build

## The ES harness, bundled with every package left an import, so it reaches
## the kit through the package's own name: the built dist/server.
build-typescript-es-harness: $(TS_STAMP)
	$(TS_BIN)/tsc --noEmit -p $(TS_DIR)/conformance-es
	$(TS_BIN)/esbuild $(TS_DIR)/conformance-es/harness.ts --bundle --platform=node --format=esm --target=node20 --packages=external --log-level=warning --outfile=$(TS_ES_HARNESS)

## Every ES case through the `/server` subpath (ADR 1.10.26af). The recipe is
## make/conformance.mk's pattern rule.
conformance-es-typescript: build-typescript build-typescript-es-harness

codegen-typescript:
	@:

check-codegen-typescript:
	@:

## vitest over the built package, both halves (TS_TEST=<filter> narrows it to
## matching files).
test-typescript: build-typescript
	cd $(TS_DIR) && ./node_modules/.bin/vitest run $(TS_TEST)

## Lint budgets, the browser types, and the web snippets: their regions, their
## types against the local package, and a build of each. Then the server
## snippets: their regions, and their types against the built dist/server.
check-typescript: build-typescript
	cd $(TS_DIR) && ./node_modules/.bin/eslint .
	$(TS_BIN)/tsc --noEmit -p $(TS_SNIPPETS)
	node $(TS_DIR)/scripts/check-snippets.mjs $(TS_SNIPPETS)
	$(TS_BIN)/tsc --noEmit -p $(TS_SERVER_SNIPPETS)
	node $(TS_DIR)/scripts/check-server-snippets.mjs $(TS_SERVER_SNIPPETS)

help-typescript:
	@echo ""
	@echo "TypeScript (@lingara/embed):"
	@echo "  make build-typescript           - dist/browser, the script-tag build and its .sri, dist/server"
	@echo "  make test-typescript            - vitest over a fresh build, both halves"
	@echo "  make check-typescript           - ESLint budgets, the player/web and server/typescript snippets"
	@echo "  make conformance-es-typescript  - Every ES case through the /server subpath (node:http)"
	@echo "  make lock-typescript            - Rewrite package-lock.json from package.json"

HELP_SECTIONS += typescript
