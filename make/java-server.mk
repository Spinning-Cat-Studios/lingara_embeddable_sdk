# java-server/ — the Java server kit, com.getlingara:lingara-embed-server-java
# (ADR 1.10.26af D2; the app kits' make/java.mk shape).
#
# java-server/ is the Gradle project :java-server in the repo-root build the
# JVM kits share. Its ES harness is :java-server:conformance-es and its
# snippets :snippets:server:java; every target below goes through the
# wrapper, so a JDK 17 is the only thing a machine needs. It generates
# nothing (its version is a build resource read from VERSION), so its codegen
# targets are no-ops that exist only because make/quality.mk's aggregates run
# them over $(LANGS).
#
# JAVA_SERVER_TEST_JDK is empty by default. Set, it runs the unit tests on
# that toolchain (-PtestJdk) while compilation stays on 17: a newest-LTS CI
# leg is `make test-java-server JAVA_SERVER_TEST_JDK=<n>`.
#
# Two make rules the conformance line depends on:
#   - java-server-harness is .PHONY, so installDist runs every time and a
#     local run never tests a stale harness; Gradle's up-to-date checks keep
#     it cheap.
#   - conformance-es-java-server is never .PHONY. GNU make skips the
#     implicit-rule search for a phony target, which would silently drop the
#     recipe of the conformance-es-% pattern rule in make/conformance.mk.

ES_LANGS += java-server

GRADLE ?= ./gradlew
GRADLE_FLAGS ?= -q --console=plain
JAVA_SERVER_TEST_JDK ?=
CONFORMANCE_HARNESS_es_java-server := java-server/conformance-es/build/install/conformance-es/bin/conformance-es

.PHONY: codegen-java-server check-codegen-java-server test-java-server java-server-harness help-java-server

codegen-java-server:
	@:

check-codegen-java-server:
	@:

## Spotless, Checkstyle and the JUnit suite (vectors, the router, the package
## shape); the harness and the snippets compile and pass the same lint.
test-java-server:
	$(GRADLE) $(GRADLE_FLAGS) $(if $(JAVA_SERVER_TEST_JDK),-PtestJdk=$(JAVA_SERVER_TEST_JDK)) :java-server:check :java-server:conformance-es:check :snippets:server:java:check

java-server-harness:
	$(GRADLE) $(GRADLE_FLAGS) :java-server:conformance-es:installDist

## Every ES case through the Java kit. No recipe, and not .PHONY (see the header).
conformance-es-java-server: java-server-harness

help-java-server:
	@echo ""
	@echo "java-server (com.getlingara:lingara-embed-server-java):"
	@echo "  make test-java-server             - Spotless, Checkstyle, JUnit, harness and snippets (JAVA_SERVER_TEST_JDK=<n>)"
	@echo "  make conformance-es-java-server   - Every ES case through the Java server kit"

HELP_SECTIONS += java-server
