// The Java conventions every Java project of the root build applies (ADR
// 1.10.26af D13; the app kits' lingara.java-conventions, 30.9.26am D8).
//
// - Compilation always uses the JDK 17 toolchain with `--release 17`. No
//   toolchain is ever provisioned, so a machine without a JDK 17 fails with
//   Gradle's own named error. Compiling on a newer JDK is never done:
//   `-Werror` would meet lint categories 17 does not have.
// - `-PtestJdk=<n>` runs the tests on that toolchain instead, leaving
//   compilation on 17: CI's newest-LTS leg.
// - `project.version` is the repository's VERSION, so no Gradle file holds a
//   version.
// - Checkstyle reads the project's own checkstyle.xml when it has one, else
//   java-server's: the harness and the snippets are held to the kit's budgets.

plugins {
    `java-library`
    checkstyle
    id("com.diffplug.spotless")
}

val catalog = extensions.getByType<VersionCatalogsExtension>().named("libs")

fun pin(name: String): String = catalog.findVersion(name).get().requiredVersion

version = rootProject.file("VERSION").readText().trim()

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(17)
    }
}

tasks.withType<JavaCompile>().configureEach {
    options.release = 17
    options.encoding = "UTF-8"
    options.compilerArgs.addAll(listOf("-Xlint:all", "-Werror"))
}

tasks.withType<Javadoc>().configureEach {
    (options as StandardJavadocDocletOptions).apply {
        encoding = "UTF-8"
        addBooleanOption("Xdoclint:none", true)
        addStringOption("Xmaxwarns", "1")
        quiet()
    }
}

spotless {
    java {
        target("src/main/java/**/*.java", "src/test/java/**/*.java")
        googleJavaFormat(pin("google-java-format"))
    }
}

checkstyle {
    toolVersion = pin("checkstyle")
    configFile = project.file("checkstyle.xml").takeIf { it.exists() } ?: rootProject.file("java-server/checkstyle.xml")
    maxWarnings = 0
}

tasks.named<Checkstyle>("checkstyleMain") {
    // Checkstyle's grammar has no module declarations.
    setSource(fileTree("src/main/java") { exclude("module-info.java") })
}

tasks.named<Checkstyle>("checkstyleTest") {
    setSource(fileTree("src/test/java"))
}

dependencies {
    testImplementation(platform(catalog.findLibrary("junit-bom").get()))
    testImplementation(catalog.findLibrary("junit-jupiter").get())
    testRuntimeOnly(catalog.findLibrary("junit-platform-launcher").get())
}

tasks.withType<Test>().configureEach {
    useJUnitPlatform()
    providers.gradleProperty("testJdk").orNull?.let { jdk ->
        javaLauncher = javaToolchains.launcherFor {
            languageVersion = JavaLanguageVersion.of(jdk)
        }
    }
}
