package com.getlingara.embed.java;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;

import com.getlingara.embed.EmbedEvent;
import com.getlingara.embed.ErrorKind;
import com.getlingara.embed.PracticePhase;
import com.getlingara.embed.PracticeSource;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import org.junit.jupiter.api.Test;

class LingaraEmbedJavaTest {
  private static final String CASE = "ek5.demo-plan-sends-nothing";
  private static final String PLAN = "8f0e2c1a-0000-4000-8000-0000000000de";

  private static boolean unanswered(EmbedEvent event) {
    return event instanceof EmbedEvent.Practice p
        && p.getSnapshot().getPhase() == PracticePhase.UNANSWERED;
  }

  /** Drains through both forms alternately until a practice event is unanswered. */
  private static List<EmbedEvent> drainToUnanswered(LingaraEmbedJava lingara)
      throws InterruptedException {
    List<EmbedEvent> seen = new ArrayList<>();
    long deadline = System.currentTimeMillis() + 2_000;
    boolean consumer = false;
    while (seen.stream().noneMatch(LingaraEmbedJavaTest::unanswered)) {
      if (System.currentTimeMillis() > deadline) {
        fail("no unanswered practice event: " + seen);
      }
      if (consumer) {
        lingara.drain(256, seen::add);
      } else {
        seen.addAll(lingara.drain(256));
      }
      consumer = !consumer;
      Thread.sleep(5);
    }
    return seen;
  }

  // 1.10.26ae AC17: the builder starts the kit against conformance-server serve; poll() is an
  // Optional; drain(int) and drain(int, Consumer) deliver events in order; a command on an
  // unknown session reaches a Java catch with getError().getKind() readable; and
  // try-with-resources closes it.
  @Test
  void builderStartsDrainsAndClosesWithTryWithResources() throws Exception {
    try (Serve serve = new Serve()) {
      serve.post("arm", CASE);
      LingaraEmbedJava kept;
      try (LingaraEmbedJava lingara =
          LingaraEmbedJava.builder()
              .baseUrl(serve.url)
              .streamOnStart(false)
              .userAgentSuffix("java-test/1.0")
              .build()) {
        kept = lingara;
        lingara.provideTokenFor("lgr_et_conformance0001", 900);
        Optional<EmbedEvent> nothingYet = lingara.poll();
        assertTrue(nothingYet.isEmpty() || nothingYet.get() != null);
        long session = lingara.practiceStart(new PracticeSource.Plan(PLAN, false));
        List<EmbedEvent> seen = drainToUnanswered(lingara);
        EmbedEvent last = seen.get(seen.size() - 1);
        assertTrue(unanswered(last), "the unanswered snapshot is the newest event seen");
        assertEquals(session, ((EmbedEvent.Practice) last).getSession());
        try {
          lingara.practiceAnswer(session + 1_000, "ni hao");
          fail("an unknown session is refused");
        } catch (LingaraEmbedJavaException e) {
          assertEquals(ErrorKind.UNKNOWN_ID, e.getError().getKind());
        }
        lingara.practiceFinish(session);
      }
      kept.close();
      String verdict = serve.post("finish", CASE);
      assertTrue(verdict.contains("\"pass\""), verdict);
    }
  }
}
