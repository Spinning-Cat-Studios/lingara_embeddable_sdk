# lingara-embed-java

The Java 17 facade over [`lingara-embed-kotlin`](../kotlin/README.md): the
same kit, with a builder for Kotlin's default arguments, `Optional`,
`Consumer`, one unchecked exception, and a visitor for exhaustive dispatch.

## Install

The kit is published to Lingara's own Maven repository, `https://maven.getlingara.com`, so your build names it beside the dependencies. The versions published before the move stay on Maven Central too; new ones are only here.

```xml
<repositories>
  <repository>
    <id>lingara</id>
    <url>https://maven.getlingara.com</url>
  </repository>
</repositories>

<dependency>
  <groupId>com.getlingara</groupId>
  <artifactId>lingara-embed-java</artifactId>
  <version>VERSION</version>
</dependency>
<!-- and the natives for the platform: lingara-embed-natives-desktop, or -natives-android -->
<dependency>
  <groupId>com.getlingara</groupId>
  <artifactId>lingara-embed-natives-desktop</artifactId>
  <version>VERSION</version>
</dependency>
```

Gradle:

```kotlin
repositories {
    exclusiveContent {
        forRepository { maven("https://maven.getlingara.com") }
        filter { includeGroup("com.getlingara") }
    }
    mavenCentral()
}

dependencies {
    implementation("com.getlingara:lingara-embed-java:VERSION")
    implementation("com.getlingara:lingara-embed-natives-desktop:VERSION")
}
```

## Use

```java
try (LingaraEmbedJava lingara = LingaraEmbedJava.builder()
        .httpTokenSource("https://my-game.example/lingara/token", Map.of("Authorization", bearer))
        .userAgentSuffix("my-game/1.4")
        .build()) {
    // once a frame, on the game thread
    lingara.drain(256, event -> LingaraEmbedJava.visit(event, visitor));
}
```

- `poll()` returns an `Optional<EmbedEvent>`; `drain(int)` a `List`, and
  `drain(int, Consumer)` hands each event over in the same order.
- The builder copies header maps in their iteration order (a `LinkedHashMap`
  keeps yours) and holds each value as a `Secret`, which never prints.
- Every command is a plain method with `long` ids; a nullable argument is one
  the core treats as absent.

## Updates

An update notice is for you, the developer, and is never drawn for the
player. `LingaraEmbedJava.builder().embedVersion("1.1.0")` names the embed
version this build was registered against, sent to the token endpoint as
`Lingara-Embed-Version`; `provideToken(token, expiresAt, update)` passes a
handoff's `update_available` on. Once per handle the visitor's
**`updateAvailable(EmbedEvent.UpdateAvailable)`** is called, beside a `log` at
`WARN`. It is a new abstract method: an existing visitor adds it.

## Errors

The core's refusals arrive as **`LingaraEmbedJavaException`**, a
`RuntimeException`: no facade method declares a checked exception.

```java
try {
    lingara.practiceAnswer(session, answer);
} catch (LingaraEmbedJavaException e) {
    if (e.getError().getKind() == ErrorKind.UNKNOWN_ID) { … }
}
```

## Exhaustive dispatch

Java 17 has no pattern matching for `switch` over a sealed hierarchy, so
implement **`EmbedEventVisitor<R>`**, one method per event type plus
`unknown` (an event from a newer core), and call
`LingaraEmbedJava.visit(event, visitor)`.

The event and snapshot types are the Kotlin kit's own classes, read with
plain getters (`event.getSession()` is a `long`).
