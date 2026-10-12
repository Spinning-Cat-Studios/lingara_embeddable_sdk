# The vendored inputs (ADR 1.10.26x D3, D4).
#
# spec/ is written by the private `make sync-embed-spec`; nothing here builds
# a view from it, because no kit generates code from the spec. The signature
# vectors are the client libraries' (E7's), copied at a release tag:
# `check-vectors` holds the file to the sha256 its SOURCE records.

VECTORS := conformance/vectors/webhook-signatures.json
VECTORS_SOURCE := conformance/vectors/SOURCE

.PHONY: check-vectors help-spec

check-vectors:
	@want=$$(sed -n 's/.* sha256:\([0-9a-f]\{64\}\)$$/\1/p' $(VECTORS_SOURCE)); \
	got=$$(shasum -a 256 $(VECTORS) | cut -d' ' -f1); \
	if [ -z "$$want" ]; then echo "✗ $(VECTORS_SOURCE) names no sha256"; exit 1; fi; \
	if [ "$$want" != "$$got" ]; then echo "✗ $(VECTORS) is sha256:$$got, but $(VECTORS_SOURCE) records sha256:$$want"; exit 1; fi; \
	echo "✓ $(VECTORS) matches $$(cat $(VECTORS_SOURCE))"

help-spec:
	@echo ""
	@echo "Spec:"
	@echo "  make check-vectors      - The signature vectors are the release SOURCE names"

HELP_SECTIONS += spec
