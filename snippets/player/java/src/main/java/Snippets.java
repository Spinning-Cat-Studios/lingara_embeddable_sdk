// The Java player kit as the documentation site shows it. Each marked region is one key; the docs
// join a key's regions in file order. No region holds a client secret: a game holds only the
// embed token its server mints.

import com.getlingara.embed.DialogueConfig;
import com.getlingara.embed.EmbedEvent;
import com.getlingara.embed.Npc;
import com.getlingara.embed.NpcTurnUpdate;
import com.getlingara.embed.PracticeSnapshot;
import com.getlingara.embed.PracticeSource;
import com.getlingara.embed.java.EmbedEventVisitor;
import com.getlingara.embed.java.LingaraEmbedJava;
import java.util.List;
import java.util.Map;

/** One screen of a Java game, the kit inside it. */
public final class Snippets implements AutoCloseable {
  private final LingaraEmbedJava lingara;
  private final EmbedEventVisitor<Void> visitor;
  private long practice = -1;
  private long npc = -1;
  private final StringBuilder bubble = new StringBuilder();

  Snippets(String session, EmbedEventVisitor<Void> visitor) {
    this.visitor = visitor;
    // lingara:begin tokenSource
    // Your server mints the embed token; the kit POSTs {} here whenever it needs one.
    lingara =
        LingaraEmbedJava.builder()
            .httpTokenSource(
                "https://my-game.example/lingara/token",
                Map.of("Authorization", "Bearer " + session))
            .userAgentSuffix("my-game/1.4")
            .build();
    // lingara:end
  }

  // lingara:begin poll
  // In render(), on the game thread: one crossing, at most 256 events, each to its visitor method.
  void render() {
    lingara.drain(256, event -> LingaraEmbedJava.visit(event, visitor));
  }

  // In dispose(), or let try-with-resources do it.
  @Override
  public void close() {
    lingara.close();
  }

  // lingara:end

  // lingara:begin practice
  void startPractice() {
    practice =
        lingara.practiceStart(new PracticeSource.Request("ordering tea", "en", "zh", 2, List.of()));
  }

  void answer(String text) {
    PracticeSnapshot graded = lingara.practiceAnswer(practice, text); // graded in the same call
    System.out.println(graded.getLastResult());
  }

  // lingara:end

  // lingara:begin dialogue
  void talkToMei(String line) {
    if (npc < 0) {
      Npc mei = new Npc("Mei", "a tea seller");
      DialogueConfig config =
          new DialogueConfig(mei, "the tea house", "en", "zh", 2, List.of(), List.of(), null);
      npc = lingara.dialogueOpen(config);
    }
    lingara.dialogueSay(npc, line);
  }

  void onNpcTurn(EmbedEvent.NpcTurn turn) {
    if (turn.getUpdate() instanceof NpcTurnUpdate.Fragment fragment) {
      bubble.append(fragment.getText());
    } else if (turn.getUpdate() instanceof NpcTurnUpdate.Finished finished) {
      bubble.setLength(0);
      bubble.append(finished.getText());
    }
  }
  // lingara:end
}
