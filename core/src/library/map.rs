//! The library's models, mapped to this crate's own types: the one place a
//! library regen can change anything, and never a binding.

use lingara::events::Event;
use lingara::models::{LessonPlan, ReactionReport};
use serde_json::Value;

use crate::api::{Plan, PlanSet, Question};
use crate::types::{LingaraEvent, LingaraEventDetail, PlanSummary, PlanWord, Reaction};

/// A library enum's wire string.
pub(crate) fn wire<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()
}

pub(crate) fn reaction(r: &ReactionReport) -> Reaction {
    Reaction {
        status: wire(&r.status),
        plan_id: r.plan_id.clone(),
        plan_status: r.plan_status.as_ref().map(wire),
        code: r.code.clone(),
        error: r.error.clone(),
    }
}

pub(crate) fn plan(p: LessonPlan) -> Plan {
    let content = p.content;
    let vocabulary = content.iter().flat_map(|c| &c.vocabulary);
    let vocabulary = vocabulary.map(|w| PlanWord { word: w.word.clone(), translation: w.translation.clone(), pronunciation: w.pronunciation.clone() });
    let summary = PlanSummary { id: p.id, title: p.title, level: p.level, source_lang: p.source_lang, target_lang: p.target_lang, vocabulary: vocabulary.collect() };
    let sets = content.into_iter().flat_map(|c| c.sets).map(|s| PlanSet {
        number: s.number,
        questions: s
            .questions
            .into_iter()
            .map(|q| Question { kind: q.type_, prompt: q.prompt, options: q.options, answer: q.answer, explanation: q.explanation, hint: q.hint })
            .collect(),
    });
    Plan { summary, status: wire(&p.status), sets: sets.collect() }
}

/// Each arm's envelope, serialised, for its raw `data`.
macro_rules! envelope {
    ($event:expr; $($arm:ident),*) => {
        match $event {
            $(Event::$arm(e) => serde_json::to_value(e).ok(),)*
            _ => None,
        }
    };
}

pub(crate) fn lingara_event(event: &Event) -> LingaraEvent {
    let envelope = envelope!(event; LessonPlanReady, LessonPlanFailed, UsageThresholdReached, WebhookTest, AppInstalled,
        AppUninstalled, AppDisabled, AppEnabled, PracticeCompleted, Unknown);
    let data = envelope.and_then(|mut v| v.get_mut("data").map(Value::take)).unwrap_or(Value::Null);
    let detail = match event {
        Event::LessonPlanReady(e) => LingaraEventDetail::LessonPlanReady {
            plan_id: e.data.plan_id.clone(),
            title: e.data.title.clone(),
            level: u8::try_from(e.data.level).unwrap_or(0),
            source_lang: e.data.source_lang.clone(),
            target_lang: e.data.target_lang.clone(),
            complete: wire(&e.data.status) == "complete",
        },
        Event::LessonPlanFailed(e) => LingaraEventDetail::LessonPlanFailed { plan_id: e.data.plan_id.clone(), reason: wire(&e.data.reason) },
        _ => LingaraEventDetail::Other,
    };
    LingaraEvent { id: event.id().to_owned(), event_type: event.event_type().to_owned(), created_at: event.created_at().to_owned(), data_json: data.to_string(), detail }
}
