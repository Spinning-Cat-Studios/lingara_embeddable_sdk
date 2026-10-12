# kotlin-server/ — the Kotlin server kit,
# com.getlingara:lingara-embed-server-kotlin (ADR 1.10.26af D2; the app kits'
# make/kotlin.mk shape).
#
# kotlin-server/ is the Gradle project :kotlin-server in the repo-root build
# the JVM kits share. Its ES harness is :kotlin-server:conformance-es (Ktor 3
# on CIO, conformance only) and its snippets :snippets:server:kotlin; every
# target below goes through the wrapper, so a JDK 17 is the only thing a
# machine needs. It generates nothing (its version is a build resource read
# from VERSION), so its codegen targets are no-ops that exist only because
# make/quality.mk's aggregates run them over $(LANGS). Its src/main is held
# to the code-health budgets by .code-health/config.toml's roots.
#
# KOTLIN_SERVER_TEST_JDK is empty by default. Set, it runs the unit tests on
# that toolchain (-PtestJdk) while compilation stays on 17.
#
# Two make rules the conformance line depends on, as in make/java-server.mk:
#   - kotlin-server-harness is .PHONY, so installDist runs every time.
#   - conformance-es-kotlin-server is never .PHONY, so it takes the recipe of
#     the conformance-es-% pattern rule in make/conformance.mk.

ES_LANGS += kotlin-server

GRADLE ?= ./gradlew
GRADLE_FLAGS ?= -q --console=plain
KOTLIN_SERVER_TEST_JDK ?=
CONFORMANCE_HARNESS_es_kotlin-server := kotlin-server/conformance-es/build/install/conformance-es/bin/conformance-es

.PHONY: codegen-kotlin-server check-codegen-kotlin-server test-kotlin-server kotlin-server-harness help-kotlin-server

codegen-kotlin-server:
	@:

check-codegen-kotlin-server:
	@:

## Spotless (ktlint) and the JUnit suite (vectors, the router, the package
## shape); the harness and the snippets compile and pass the same lint.
test-kotlin-server:
	$(GRADLE) $(GRADLE_FLAGS) $(if $(KOTLIN_SERVER_TEST_JDK),-PtestJdk=$(KOTLIN_SERVER_TEST_JDK)) :kotlin-server:check :kotlin-server:conformance-es:check :snippets:server:kotlin:check

kotlin-server-harness:
	$(GRADLE) $(GRADLE_FLAGS) :kotlin-server:conformance-es:installDist

## Every ES case through the Kotlin kit. No recipe, and not .PHONY (see the header).
conformance-es-kotlin-server: kotlin-server-harness

help-kotlin-server:
	@echo ""
	@echo "kotlin-server (com.getlingara:lingara-embed-server-kotlin):"
	@echo "  make test-kotlin-server             - Spotless, JUnit, harness and snippets (KOTLIN_SERVER_TEST_JDK=<n>)"
	@echo "  make conformance-es-kotlin-server   - Every ES case through the Kotlin server kit"

HELP_SECTIONS += kotlin-server
