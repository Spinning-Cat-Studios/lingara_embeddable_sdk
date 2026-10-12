package com.getlingara.embed.java;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.getlingara.embed.CloseReason;
import com.getlingara.embed.EmbedError;
import com.getlingara.embed.EmbedEvent;
import com.getlingara.embed.ErrorKind;
import com.getlingara.embed.LingaraEvent;
import com.getlingara.embed.LingaraEventDetail;
import com.getlingara.embed.LogLevel;
import com.getlingara.embed.NpcTurnUpdate;
import com.getlingara.embed.PracticePhase;
import com.getlingara.embed.PracticeSnapshot;
import com.getlingara.embed.StreamState;
import com.getlingara.embed.TokenReason;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.util.Arrays;
import java.util.List;
import java.util.Set;
import java.util.stream.Collectors;
import org.junit.jupiter.api.Test;

class VisitorCoverageTest {
  private static String methodFor(Class<?> subclass) {
    String name = subclass.getSimpleName();
    return Character.toLowerCase(name.charAt(0)) + name.substring(1);
  }

  private static List<EmbedEvent> oneOfEach() {
    EmbedError error = new EmbedError(ErrorKind.NETWORK, "reset", null, null, null, true, null);
    PracticeSnapshot snapshot =
        new PracticeSnapshot(PracticePhase.LOADING, null, 0, 0, 0, 0, null, null, null);
    return List.of(
        new EmbedEvent.TokenNeeded(1, TokenReason.INITIAL),
        new EmbedEvent.Stream(StreamState.Live.INSTANCE, null),
        new EmbedEvent.Lingara(
            new LingaraEvent("e1", "game.x", "t", "{}", LingaraEventDetail.Other.INSTANCE)),
        new EmbedEvent.WorldSent(2, "e2", null),
        new EmbedEvent.Practice(3, snapshot),
        new EmbedEvent.NpcTurn(4, 5, NpcTurnUpdate.Opened.INSTANCE),
        new EmbedEvent.Error(error),
        new EmbedEvent.Log(LogLevel.WARN, "t", "m"),
        new EmbedEvent.Closed(CloseReason.REQUESTED),
        new EmbedEvent.Unknown("future_thing", "{\"type\":\"future_thing\"}"));
  }

  // 1.10.26ae AC9: EmbedEventVisitor has a method for every sealed subclass of EmbedEvent, and
  // LingaraEmbedJava.visit dispatches each subclass to its own method.
  @Test
  void everyEventSubclassHasAVisitorMethod() {
    Set<Class<?>> subclasses =
        Arrays.stream(EmbedEvent.class.getDeclaredClasses())
            .filter(c -> EmbedEvent.class.isAssignableFrom(c))
            .filter(c -> !Modifier.isAbstract(c.getModifiers()))
            .collect(Collectors.toSet());
    Set<String> methods =
        Arrays.stream(EmbedEventVisitor.class.getDeclaredMethods())
            .map(Method::getName)
            .collect(Collectors.toSet());
    for (Class<?> subclass : subclasses) {
      assertTrue(methods.contains(methodFor(subclass)), "no visitor method for " + subclass);
    }
    assertEquals(subclasses.size(), methods.size(), "one method per subclass, no more");

    List<EmbedEvent> events = oneOfEach();
    assertEquals(
        subclasses,
        events.stream().map(Object::getClass).collect(Collectors.toSet()),
        "the fixture covers every subclass");
    InvocationHandler named = (proxy, method, args) -> method.getName();
    @SuppressWarnings("unchecked")
    EmbedEventVisitor<String> visitor =
        (EmbedEventVisitor<String>)
            Proxy.newProxyInstance(
                getClass().getClassLoader(), new Class<?>[] {EmbedEventVisitor.class}, named);
    for (EmbedEvent event : events) {
      assertEquals(methodFor(event.getClass()), LingaraEmbedJava.visit(event, visitor));
    }
  }
}
