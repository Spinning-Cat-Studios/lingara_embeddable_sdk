# The aggregates over $(LANGS) (ADR 1.10.26x D6).
#
# Read after every make/<area>.mk (see the Makefile): these prerequisite
# lists are expanded when this file is read. A kit never edits this file;
# appending itself to EK_LANGS or ES_LANGS is the whole of joining.

.PHONY: codegen check-codegen test test-tools check-library-floor check-publishable help-quality

## Regenerate what each kit derives from the repo itself: binding output
## (UniFFI, cbindgen, gdext) and the files that carry VERSION.
codegen: $(addprefix codegen-,$(LANGS))

## Every committed generated file equals a fresh codegen.
check-codegen: $(addprefix check-codegen-,$(LANGS))

test: test-tools $(addprefix test-,$(LANGS))

## The conformance server's own tests.
test-tools:
	$(CARGO) test --workspace $(WORKSPACE_TEST_EXCLUDES)

## One LIBRARY_FLOOR in every manifest that depends on a library (ADR 1.10.26y D1).
check-library-floor:
	bash tools/library-floor/check_test.sh
	bash tools/library-floor/check.sh

## What a published snapshot must pass: run before every publish. The last
## is release-manifest check: languages.toml against the tree (ADR 1.10.26ag).
check-publishable: check-vectors check-conformance-coverage check-codegen check-library-floor check-release-tree

help-quality:
	@echo ""
	@echo "Quality:"
	@echo "  make codegen             - Regenerate every kit: $(or $(LANGS),none yet)"
	@echo "  make check-codegen       - Every kit's generated files are current"
	@echo "  make test                - The conformance server's tests and every kit's"
	@echo "  make check-library-floor - One LIBRARY_FLOOR in every library manifest"
	@echo "  make check-publishable   - What a publish must pass"

HELP_SECTIONS += quality
