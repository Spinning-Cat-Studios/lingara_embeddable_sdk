//! v1 is read as v2, and written back as v1 when v1 can say it (ADR 3.11.26g
//! §2, §11.2). The write rule lives here once; the canonical writer and
//! Backend's store both call [`write_schema`], and the C# and GDScript
//! builders' copies are pinned to its bytes.

use crate::Document;
use crate::ir::{DialogueIr, Ending, SCHEMA_V1, SCHEMA_V2};
use crate::limits::V1_ENDINGS;

/// The schema a writer emits for `doc`: `dialogue.v1` if and only if it is
/// static, every ending is `success` or `failure` with no label, no NPC has a
/// `disposition` and no checkpoint a `steer_to`. A procedural document is
/// always `dialogue.v2`. `DialogueIr.schema` is not consulted.
pub fn write_schema(doc: &Document) -> &'static str {
    match doc {
        Document::Static(ir) => static_schema(ir),
        Document::Procedural(_) => SCHEMA_V2,
    }
}

/// [`write_schema`] for a static graph, without building a [`Document`].
pub fn static_schema(ir: &DialogueIr) -> &'static str {
    let v1_endings = ir.endings.iter().all(|e| V1_ENDINGS.contains(&e.id.as_str()) && e.label.is_none());
    let stance = ir.npcs.iter().any(|npc| npc.disposition.is_some());
    let steer = ir.checkpoints.iter().any(|checkpoint| checkpoint.steer_to.is_some());
    if v1_endings && !stance && !steer { SCHEMA_V1 } else { SCHEMA_V2 }
}

/// The endings every reader gives a v1 document.
pub fn v1_endings() -> Vec<Ending> {
    V1_ENDINGS.iter().map(|id| Ending::bare(id)).collect()
}

/// A freshly read v1 document, opened as v2: `[success, failure]`, nothing else.
pub(crate) fn read_v1(mut ir: DialogueIr) -> DialogueIr {
    ir.endings = v1_endings();
    ir
}
