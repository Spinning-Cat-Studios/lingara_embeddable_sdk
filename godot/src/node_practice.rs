//! `LingaraEmbed`'s practice methods (ADR 1.10.26ab D3). K1's answer
//! methods are synchronous: each returns the new snapshot **and** emits it
//! as `practice_changed`, so a panel connected only to the signal sees
//! every state, and code that prefers return values never waits a frame.

use godot::prelude::*;
use serde_json::Value;

use crate::binding::Returns;
use crate::convert::arg;
use crate::embed_node::LingaraEmbed;
use crate::variant::from_dictionary;

/// The session to emit for, when the argument narrows; a negative one is
/// refused by the command itself.
fn moving(session: i64) -> Returns {
    Returns::Practice(arg::<u64>("session", Value::from(session)).ok())
}

#[godot_api(secondary)]
impl LingaraEmbed {
    /// K1's `PracticeSource`: `request`, `create`, or `plan` with `report`
    /// (`false` for a demo plan). Returns the session id.
    #[func]
    pub fn practice_start(&mut self, source: VarDictionary) -> i64 {
        self.flat("practice_start", vec![("source", from_dictionary(&source))], Returns::Id).to()
    }

    #[func]
    pub fn practice_answer(&mut self, session: i64, answer: GString) -> VarDictionary {
        let fields = vec![("session", session.into()), ("answer", answer.to_string().into())];
        self.flat("practice_answer", fields, moving(session)).to()
    }

    #[func]
    pub fn practice_choose(&mut self, session: i64, option: i64) -> VarDictionary {
        self.flat("practice_choose", vec![("session", session.into()), ("option", option.into())], moving(session)).to()
    }

    #[func]
    pub fn practice_reveal(&mut self, session: i64) -> VarDictionary {
        self.flat("practice_reveal", vec![("session", session.into())], moving(session)).to()
    }

    #[func]
    pub fn practice_self_grade(&mut self, session: i64, correct: bool) -> VarDictionary {
        self.flat("practice_self_grade", vec![("session", session.into()), ("correct", correct.into())], moving(session)).to()
    }

    #[func]
    pub fn practice_next(&mut self, session: i64) -> VarDictionary {
        self.flat("practice_next", vec![("session", session.into())], moving(session)).to()
    }

    #[func]
    pub fn practice_finish(&mut self, session: i64) -> VarDictionary {
        self.flat("practice_finish", vec![("session", session.into())], moving(session)).to()
    }

    /// Reads only: nothing is emitted.
    #[func]
    pub fn practice_snapshot(&mut self, session: i64) -> VarDictionary {
        self.flat("practice_snapshot", vec![("session", session.into())], Returns::Practice(None)).to()
    }
}
