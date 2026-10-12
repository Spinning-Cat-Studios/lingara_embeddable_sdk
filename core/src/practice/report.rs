//! The one report a session sends (D7, EK5): `world.practice_completed`
//! with B2's body, every `(set_number, question)` pair once, in plan order.

use serde_json::{Value, json};

use crate::limits;
use crate::practice::machine::Session;
use crate::types::PracticeOutcome;

/// `(set_number, question, outcome)`: the plan's own set number, and the
/// 0-based index within that set.
pub(crate) type Item = (u8, usize, PracticeOutcome);

/// Every graded slot, capped at `MAX_PRACTICE_ITEMS`: `skipped` items go
/// first (the latest first), then the list is cut in plan order. True when
/// the cap cut anything.
pub(crate) fn items(session: &Session) -> (Vec<Item>, bool) {
    let Some(plan) = &session.plan else { return (Vec::new(), false) };
    let mut items: Vec<Item> = plan
        .sets
        .iter()
        .zip(&session.outcomes)
        .flat_map(|(set, outcomes)| outcomes.iter().enumerate().filter_map(move |(i, o)| o.map(|o| (set.number, i, o))))
        .collect();
    let over = items.len().saturating_sub(limits::MAX_PRACTICE_ITEMS);
    let mut to_drop = over;
    while to_drop > 0 {
        match items.iter().rposition(|(_, _, o)| *o == PracticeOutcome::Skipped) {
            Some(at) => {
                items.remove(at);
                to_drop -= 1;
            }
            None => break,
        }
    }
    items.truncate(limits::MAX_PRACTICE_ITEMS);
    (items, over > 0)
}

pub(crate) fn body(session: &Session, items: &[Item]) -> Value {
    let plan_id = session.plan.as_ref().map(|p| p.summary.id.clone()).unwrap_or_default();
    let items: Vec<Value> = items.iter().map(|(set, question, outcome)| json!({ "set_number": set, "question": question, "outcome": outcome })).collect();
    json!({
        "type": "world.practice_completed",
        "data": { "plan_id": plan_id, "items": items, "completed": session.completed },
    })
}

/// The report's idempotency key: one per session, so a retry, or a re-send
/// after a retryable failure, is one event.
pub(crate) fn key(session: &Session) -> String {
    format!("pc:{}", session.key)
}
