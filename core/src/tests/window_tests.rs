use crate::dialogue::window::Window;
use crate::types::{Speaker, WindowEntry};

fn texts(w: &Window) -> Vec<(Speaker, String)> {
    w.entries().iter().map(|e| (e.speaker, e.text.clone())).collect()
}

// 1.10.26y AC13
#[test]
fn the_window_keeps_twelve_entries_and_cuts_replies() {
    let mut w = Window::default();
    w.push_turn("hello", "你好");
    assert_eq!(texts(&w), vec![(Speaker::Player, "hello".into()), (Speaker::Npc, "你好".into())]);

    // The reply is cut to 500 scalar values: 600 CJK characters are 1 800 bytes.
    w.push_turn("say a lot", &"长".repeat(600));
    assert_eq!(w.entries()[3].text.chars().count(), 500);

    // An empty reply appends only the line.
    w.push_turn("are you there?", "   ");
    assert_eq!(w.entries().len(), 5);
    assert_eq!(w.entries()[4].speaker, Speaker::Player);

    // Oldest first, to 12 entries, with no character total.
    for i in 0..10 {
        w.push_turn(&format!("line {i}"), &"x".repeat(500));
    }
    assert_eq!(w.entries().len(), 12);
    assert_eq!(w.entries()[0].text, "line 4");
    assert_eq!(w.entries()[11].text, "x".repeat(500));
    assert!(w.entries().iter().map(|e| e.text.len()).sum::<usize>() > 3_000, "no character trim");

    // A supplied window gets the same rule.
    let mut supplied: Vec<WindowEntry> = (0..14).map(|i| WindowEntry { speaker: Speaker::Player, text: format!("{i}") }).collect();
    supplied.push(WindowEntry { speaker: Speaker::Npc, text: " ".into() });
    let w = Window::from_supplied(supplied);
    assert_eq!((w.entries().len(), w.entries()[0].text.as_str()), (12, "2"));
}
