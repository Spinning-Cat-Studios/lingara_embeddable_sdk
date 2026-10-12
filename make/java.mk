# java/ — the Java 17 facade, com.getlingara:lingara-embed-java (ADR 1.10.26ae
# D4), the Gradle project :java over :kotlin in the repo-root build. It
# generates nothing, so its codegen targets are no-ops that exist only
# because make/quality.mk's aggregates run them over $(LANGS). Its tests and
# its EK harness load the host library make/kotlin.mk builds.
#
# conformance-ek-java is never .PHONY, so it takes the recipe of the
# conformance-ek-% pattern rule in make/conformance.mk.

EK_LANGS += java

CONFORMANCE_HARNESS_ek_java := java/conformance/build/install/conformance/bin/conformance

.PHONY: codegen-java check-codegen-java test-java java-harness help-java

codegen-java:
	@:

check-codegen-java:
	@:

## Spotless, Checkstyle and the JUnit suite against the host library; the
## harness and the snippets compile and pass the same lint.
test-java: kotlin-natives-host
	$(GRADLE) $(GRADLE_FLAGS) :java:check :java:conformance:check :snippets:player:java:check

java-harness: kotlin-natives-host
	$(GRADLE) $(GRADLE_FLAGS) :java:conformance:installDist

## Every EK case through LingaraEmbedJava. No recipe, and not .PHONY (see the header).
conformance-ek-java: java-harness

help-java:
	@echo ""
	@echo "java (com.getlingara:lingara-embed-java):"
	@echo "  make test-java                   - Spotless, Checkstyle and JUnit on the host library"
	@echo "  make conformance-ek-java         - Every EK case through the Java facade"

HELP_SECTIONS += java
