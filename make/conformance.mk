# The conformance tiers (ADR 1.10.26x D5, D6).
#
# Read after every make/<area>.mk (see the Makefile), so EK_LANGS and
# ES_LANGS are complete when the prerequisites below expand. A binding joins
# by setting CONFORMANCE_HARNESS_<tier>_<lang> to the command that starts its
# harness; `check-conformance-coverage` fails a list member without one. A
# binding whose engine cannot run in public CI still joins: its harness drives
# the layer beneath it, and its ADR names that layer.

.PHONY: conformance check-conformance-coverage help-conformance

## Every landed binding's harness against every case of its tier.
conformance: $(addprefix conformance-ek-,$(EK_LANGS)) $(addprefix conformance-es-,$(ES_LANGS))
	@if [ -z "$(strip $(EK_LANGS) $(ES_LANGS))" ]; then echo "no binding has landed yet: nothing to run"; fi

conformance-ek-%:
	$(CONFORMANCE_SERVER) run --tier ek --lang $* --registry $(SPEC_REGISTRY) -- $(CONFORMANCE_HARNESS_ek_$*)

conformance-es-%:
	$(CONFORMANCE_SERVER) run --tier es --lang $* --registry $(SPEC_REGISTRY) -- $(CONFORMANCE_HARNESS_es_$*)

# The harness variables a list member is missing. `conformance-<tier>-%` is a
# pattern rule, so every name "has" the target; only the variable says a
# harness exists.
CONFORMANCE_MISSING = $(strip \
  $(foreach l,$(EK_LANGS),$(if $(strip $(CONFORMANCE_HARNESS_ek_$(l))),,CONFORMANCE_HARNESS_ek_$(l))) \
  $(foreach l,$(ES_LANGS),$(if $(strip $(CONFORMANCE_HARNESS_es_$(l))),,CONFORMANCE_HARNESS_es_$(l))))

## Both tiers' cases against their lists and the frozen current, and every
## list member has a harness.
check-conformance-coverage:
	@if [ -n "$(CONFORMANCE_MISSING)" ]; then echo "✗ not set: $(CONFORMANCE_MISSING)"; exit 1; fi
	$(CONFORMANCE_SERVER) check-coverage --tier ek --registry $(SPEC_REGISTRY) --langs "$(EK_LANGS)"
	$(CONFORMANCE_SERVER) check-coverage --tier es --registry $(SPEC_REGISTRY) --langs "$(ES_LANGS)"
	@echo "✓ every binding has a harness: ek $(or $(EK_LANGS),none yet) · es $(or $(ES_LANGS),none yet)"

help-conformance:
	@echo ""
	@echo "Conformance:"
	@echo "  make conformance                 - Every binding against its tier: ek $(or $(EK_LANGS),none) · es $(or $(ES_LANGS),none)"
	@echo "  make conformance-ek-<lang>       - One player binding"
	@echo "  make conformance-es-<lang>       - One server kit"
	@echo "  make check-conformance-coverage  - Both tiers' cases, lists and harnesses"

HELP_SECTIONS += conformance
