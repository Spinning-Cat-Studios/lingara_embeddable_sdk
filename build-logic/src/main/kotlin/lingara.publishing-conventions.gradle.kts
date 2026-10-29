// Publishing to the Maven repository we host, maven.getlingara.com (Generator
// ADR 23.10.26a §2, applied here by 23.10.26b; Central's metering retired it
// for new versions), in the clients' shape (their 29.9.26v D6a, ported by the
// app kits' 30.9.26am D8 and here by ADR 1.10.26af D13).
//
// - com.vanniktech.maven.publish still supplies the POM, the sources and
//   javadoc jars and the signatures, signing from in-memory keys, so no
//   keyring file exists anywhere. The keys arrive as
//   ORG_GRADLE_PROJECT_signingInMemoryKey and ...signingInMemoryKeyPassword.
// - The `lingara` repository is the R2 bucket behind maven.getlingara.com,
//   written through Gradle's own S3 transport. That transport downloads the
//   artifact's maven-metadata.xml, adds the version and uploads it again — the
//   merge a sync of a fresh staging repo would get wrong. The R2 endpoint
//   arrives as -Dorg.gradle.s3.endpoint from release.yml, so no build file
//   carries the account id.
// - The artifact is `lingara-embed-<project name>`, except that a `-server`
//   project reads server first: java-server is lingara-embed-server-java and
//   kotlin-server lingara-embed-server-kotlin (ADR 1.10.26af D2), while K7's
//   kotlin is lingara-embed-kotlin.
// - Java's javadoc jar is real javadoc output; Kotlin's is the plugin's empty
//   jar, kept so the published layout matches the versions on Central.
// - The `staging` file repository under the root build/ is what staging's
//   dry run publishes to: only a repository publish writes the .md5 and .sha1
//   files, which publishToMavenLocal does not.
// - ADR 1.10.26ag D6a applies it to all six coordinates: K7's four join V1's
//   two. natives-android is an Android library, published as its single
//   release variant (an AAR) with an empty javadoc jar, as the natives
//   desktop jar's is (it holds no classes, only JNA's resource directories).
// - Every jar carries the root LICENSE at META-INF/LICENSE, which
//   tools/release/check-package.sh requires of a package with natives inside.

import com.vanniktech.maven.publish.AndroidSingleVariantLibrary
import com.vanniktech.maven.publish.JavaLibrary
import com.vanniktech.maven.publish.JavadocJar
import com.vanniktech.maven.publish.KotlinJvm
import com.vanniktech.maven.publish.SourcesJar

plugins {
    id("com.vanniktech.maven.publish")
}

// The coordinates are read now, so the version must be set before they are.
// The Java and Kotlin conventions set it; natives-android applies neither.
version = rootProject.file("VERSION").readText().trim()

val artifact =
    "lingara-embed-" +
        if (project.name.endsWith("-server")) "server-" + project.name.removeSuffix("-server") else project.name

// Signing needs a key, which only the dry run (an ephemeral one) and the
// release publish have; publishToMavenLocal for kotlin/shape signs nothing
// (1.10.26ag D6a). Central refused an unsigned upload; R2 does not, so the
// guard below refuses one before it leaves (Generator ADR 23.10.26b).
val signingKey = providers.gradleProperty("signingInMemoryKey")

tasks.withType<PublishToMavenRepository>().configureEach {
    val toLingara = name.endsWith("ToLingaraRepository")
    val signed = signingKey.isPresent
    doFirst {
        check(!toLingara || signed) { "refusing an unsigned upload to maven.getlingara.com: set signingInMemoryKey" }
    }
}

mavenPublishing {
    if (signingKey.isPresent) {
        signAllPublications()
    }
    coordinates("com.getlingara", artifact, project.version.toString())
    pom {
        name = artifact
        description = provider { project.description ?: artifact }
        url = "https://github.com/Spinning-Cat-Studios/lingara_embeddable_sdk"
        licenses {
            license {
                name = "MIT"
                url = "https://opensource.org/license/mit"
            }
        }
        scm {
            url = "https://github.com/Spinning-Cat-Studios/lingara_embeddable_sdk"
            connection = "scm:git:https://github.com/Spinning-Cat-Studios/lingara_embeddable_sdk.git"
            developerConnection = "scm:git:ssh://git@github.com/Spinning-Cat-Studios/lingara_embeddable_sdk.git"
        }
        developers {
            developer {
                id = "spinning-cat-studios"
                name = "Spinning Cat Studios"
                email = "support@getlingara.com"
            }
        }
    }
}

pluginManager.withPlugin("org.jetbrains.kotlin.jvm") {
    mavenPublishing {
        configure(KotlinJvm(javadocJar = JavadocJar.Empty(), sourcesJar = true))
    }
}

pluginManager.withPlugin("java-library") {
    if (!pluginManager.hasPlugin("org.jetbrains.kotlin.jvm")) {
        val javadoc = if (project.name.startsWith("natives-")) JavadocJar.Empty() else JavadocJar.Javadoc()
        mavenPublishing {
            configure(JavaLibrary(javadocJar = javadoc, sourcesJar = true))
        }
    }
}

pluginManager.withPlugin("com.android.library") {
    mavenPublishing {
        configure(AndroidSingleVariantLibrary(javadocJar = JavadocJar.Empty(), sourcesJar = SourcesJar.Sources(), variant = "release"))
    }
}

tasks.withType<Jar>().configureEach {
    from(rootProject.file("LICENSE")) { into("META-INF") }
}

publishing {
    repositories {
        maven {
            name = "staging"
            url = uri(rootProject.layout.buildDirectory.dir("staging-repo"))
        }
        maven {
            name = "lingara"
            url = uri("s3://lingara-maven")
            credentials(AwsCredentials::class) {
                accessKey = providers.environmentVariable("R2_MAVEN_ACCESS_KEY_ID").orNull
                secretKey = providers.environmentVariable("R2_MAVEN_SECRET_ACCESS_KEY").orNull
            }
        }
    }
}
