# Shared variables (ADR 1.10.26x D6).
#
# Two lists, not one. A player binding appends itself to EK_LANGS, a server
# kit to ES_LANGS, and a directory holding both (core, dotnet) to both. Each
# make/<area>.mk also defines codegen-<lang>, check-codegen-<lang> and
# test-<lang>, and sets CONFORMANCE_HARNESS_<tier>_<lang> for each list it
# joins. LANGS is their sorted union, for the aggregates in make/quality.mk.

EK_LANGS :=
ES_LANGS :=
LANGS = $(sort $(EK_LANGS) $(ES_LANGS))
HELP_SECTIONS :=

CARGO ?= cargo
# Workspace members `test-tools` leaves to their own kit's lane, as
# `--exclude <package>` flags: a kit whose MSRV is above the workspace
# toolchain tests them on its own (make/bevy.mk, ADR 1.10.26z).
WORKSPACE_TEST_EXCLUDES :=
CONFORMANCE_SERVER := $(CARGO) run -q -p lingara-embed-conformance-server --
SPEC_REGISTRY := spec/versions.toml
