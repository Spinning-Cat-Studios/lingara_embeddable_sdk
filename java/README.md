# lingara-embed-java

The Java 17 facade over [`lingara-embed-kotlin`](../kotlin/README.md): the
same kit, with a builder for Kotlin's default arguments, `Optional`,
`Consumer`, one unchecked exception, and a visitor for exhaustive dispatch.

## Install

```xml
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
