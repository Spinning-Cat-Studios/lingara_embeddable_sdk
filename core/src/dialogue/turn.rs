//! One turn in flight (D8): `Opened` when the headers are in, B3's frames
//! in order, and, for a graph, N2's `transition` held until `done` (ADR
//! 28.10.26f D5).

use std::sync::Arc;

use serde_json::Value;

use crate::api::TurnFrame;
use crate::dialogue::graph;
use crate::error::EmbedError;
use crate::shared::Shared;
use crate::types::{DialogueId, DialoguePhase, EmbedEvent, NpcTurnUpdate, RequestId};

/// What a turn has received so far: the reply, and the transition it holds.
#[derive(Default)]
struct Received {
    reply: String,
    held: Option<(String, String)>,
}

async fn on_frame(shared: &Shared, dialogue: DialogueId, request: RequestId, frame: TurnFrame, got: &mut Received) {
    let update = |update| EmbedEvent::NpcTurn { dialogue, request, update };
    match frame {
        TurnFrame::Delta(text) => {
            got.reply.push_str(&text);
            shared.outbox.send(update(NpcTurnUpdate::Fragment { text })).await;
        }
        TurnFrame::Notice { code, message } => shared.outbox.send(update(NpcTurnUpdate::Notice { code, message })).await,
        TurnFrame::Transition { edge, to } if got.held.is_none() => got.held = Some((edge, to)),
        TurnFrame::Transition { edge, .. } => shared.diag.warn(graph::TARGET, format!("a second transition in one turn (over `{edge}`) was ignored")),
        TurnFrame::Other => {}
    }
}

/// One turn: `Opened` when the headers are in, then the frames in order.
pub(super) async fn turn(shared: Arc<Shared>, dialogue: DialogueId, request: RequestId, body: Value) {
    let mut frames = match shared.api.dialogue_turn(body).await {
        Ok(frames) => frames,
        Err(error) => return end_turn(&shared, dialogue, request, Err(error)).await,
    };
    let current = shared.dialogues.with(dialogue, |d| {
        let current = d.turn.as_ref().is_some_and(|t| t.request == request);
        if current {
            d.phase = DialoguePhase::Streaming;
        }
        Ok(current)
    });
    if current != Ok(true) {
        return;
    }
    shared.outbox.send(EmbedEvent::NpcTurn { dialogue, request, update: NpcTurnUpdate::Opened }).await;
    let mut got = Received::default();
    let result = loop {
        match frames.next().await {
            Some(Ok(frame)) => on_frame(&shared, dialogue, request, frame, &mut got).await,
            Some(Err(error)) => break Err(error),
            None => break Ok(got),
        }
    };
    end_turn(&shared, dialogue, request, result).await;
}

/// Back to `Idle`. Only a finished turn moves the window, and only it
/// crosses its held transition, after `Finished`; a failed one leaves both as
/// they were, so saying the line again sends the same state.
async fn end_turn(shared: &Shared, dialogue: DialogueId, request: RequestId, result: Result<Received, EmbedError>) {
    let ended = shared.dialogues.with(dialogue, |d| {
        let Some(turn) = d.turn.take_if(|t| t.request == request) else { return Ok(None) };
        d.phase = DialoguePhase::Idle;
        let mut steps = Vec::new();
        if let Ok(got) = &result {
            d.window.push_turn(&turn.line, &got.reply);
            if let (Some(graph), Some((edge, to))) = (d.graph.as_mut(), &got.held) {
                steps = graph.apply_judged(&turn.sent, edge, to);
            }
        }
        steps.extend(d.settle());
        Ok(Some(steps))
    });
    let Ok(Some(steps)) = ended else { return };
    let event = match result {
        Ok(got) => EmbedEvent::NpcTurn { dialogue, request, update: NpcTurnUpdate::Finished { text: got.reply } },
        Err(error) => EmbedEvent::Error { error: error.with_request(request) },
    };
    shared.outbox.send(event).await;
    graph::emit(shared, dialogue, steps);
}
