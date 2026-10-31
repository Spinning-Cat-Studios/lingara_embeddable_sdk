//! A case's `host:` script on the Rust surface (D12). Each step goes
//! through `Embed`'s public methods only. A step the core refuses (a
//! `provide_token` with a client token, a command out of bounds) becomes an
//! `error` event in the harness's view, so a case asserts a refusal with
//! the same `poll_until` it uses for everything else.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use lingara_embed::{Command, CommandOutcome, Embed, EmbedConfig, EmbedError};
use serde_json::{Value, json};

use crate::matching::contains;

/// How long `close` waits for the one `Closed`.
const CLOSE_WITHIN: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct Run {
    pub problems: Vec<String>,
    /// The events the `poll_until` steps matched, in order.
    pub matched: Vec<Value>,
    embed: Option<Embed>,
    /// Refusals, seen before the queue.
    local: VecDeque<Value>,
    last_id: Option<u64>,
}

pub fn run(server: &str, case: &Value) -> Run {
    let id = case["id"].as_str().unwrap_or_default().to_owned();
    let mut run = Run::default();
    for (n, step) in case["host"].as_array().into_iter().flatten().enumerate() {
        let Some((name, body)) = step.as_object().and_then(|o| o.iter().next()) else { continue };
        if let Err(problem) = run.step(server, name, body) {
            run.problems.push(format!("{id}: host[{n}].{name}: {problem}"));
            break;
        }
    }
    if let Some(embed) = run.embed.take() {
        embed.close_and_wait(1_000);
    }
    run
}

fn refusal(error: &EmbedError) -> Value {
    json!({ "type": "error", "error": error })
}

fn substitute(value: &mut Value, last: Option<u64>) {
    match value {
        Value::String(s) if s == "$last" => *value = last.map_or(Value::Null, Value::from),
        Value::Object(map) => map.values_mut().for_each(|v| substitute(v, last)),
        Value::Array(items) => items.iter_mut().for_each(|v| substitute(v, last)),
        _ => {}
    }
}

impl Run {
    fn embed(&self) -> Result<&Embed, String> {
        self.embed.as_ref().ok_or_else(|| "no start step ran".to_owned())
    }

    fn step(&mut self, server: &str, name: &str, body: &Value) -> Result<(), String> {
        match name {
            "start" => self.start(server, body),
            "provide_token" => self.provide(body),
            "command" => self.command(body),
            "poll_until" => self.poll_until(body),
            "expect_no_event" => self.expect_no_event(body),
            "stop_polling" => {
                std::thread::sleep(Duration::from_millis(body["for_ms"].as_u64().unwrap_or(0)));
                Ok(())
            }
            "close" => self.close(),
            other => Err(format!("unknown step {other}")),
        }
    }

    fn start(&mut self, server: &str, body: &Value) -> Result<(), String> {
        let mut config = body.get("config").cloned().unwrap_or_else(|| json!({}));
        config["base_url"] = server.into();
        let config: EmbedConfig = serde_json::from_value(config).map_err(|e| format!("not an EmbedConfig: {e}"))?;
        let embed = Embed::start(config).map_err(|e| e.to_string())?;
        if let Some(token) = body["initial_token"].as_str().filter(|t| *t != "none") {
            embed.provide_token_for(token, 900).map_err(|e| e.to_string())?;
        }
        self.embed = Some(embed);
        Ok(())
    }

    /// A `Command` in its JSON shape. Any value `"$last"` in it is the id
    /// the latest `practice_start`, `dialogue_open` or `dialogue_load` answered with.
    fn command(&mut self, body: &Value) -> Result<(), String> {
        let mut body = body.clone();
        substitute(&mut body, self.last_id);
        let command: Command = serde_json::from_value(body).map_err(|e| format!("not a Command: {e}"))?;
        let opens = matches!(command, Command::PracticeStart { .. } | Command::DialogueOpen { .. } | Command::DialogueLoad { .. });
        match self.embed()?.execute(command) {
            Ok(CommandOutcome::Id { id }) if opens => self.last_id = Some(id),
            Ok(_) => {}
            Err(e) => self.local.push_back(refusal(&e)),
        }
        Ok(())
    }

    fn provide(&mut self, body: &Value) -> Result<(), String> {
        let embed = self.embed()?;
        let token = body["token"].as_str().unwrap_or_default();
        if let Some(update) = body.get("update_available") {
            // EK13: the notice rides the JSON command, as every JSON binding sends it.
            let kind = if body.get("expires_at").is_some() { "provide_token" } else { "provide_token_for" };
            let mut command = body.clone();
            command["type"] = kind.into();
            command["update_available"] = update.clone();
            let command: Command = serde_json::from_value(command).map_err(|e| format!("not a Command: {e}"))?;
            if let Err(e) = embed.execute(command) {
                self.local.push_back(refusal(&e));
            }
            return Ok(());
        }
        let result = match (body["expires_at"].as_str(), body["expires_in_secs"].as_u64()) {
            (Some(at), _) => embed.provide_token(token, at),
            (None, Some(secs)) => embed.provide_token_for(token, u32::try_from(secs).unwrap_or(u32::MAX)),
            (None, None) => return Err("no expires_at or expires_in_secs".into()),
        };
        if let Err(e) = result {
            self.local.push_back(refusal(&e));
        }
        Ok(())
    }

    /// The next event, a refusal first.
    fn next(&mut self) -> Option<Value> {
        if let Some(local) = self.local.pop_front() {
            return Some(local);
        }
        let event = self.embed.as_ref()?.poll_event()?;
        serde_json::to_value(event).ok()
    }

    /// Drains until an event matches or `deadline` passes.
    fn find(&mut self, event: &str, wanted: &Value, deadline: Instant) -> Option<Value> {
        while Instant::now() < deadline {
            match self.next() {
                Some(seen) if seen["type"] == event && contains(&seen, wanted) => return Some(seen),
                Some(_) => {}
                None => std::thread::sleep(Duration::from_millis(5)),
            }
        }
        None
    }

    fn poll_until(&mut self, body: &Value) -> Result<(), String> {
        let (event, wanted) = (body["event"].as_str().unwrap_or_default(), &body["match"]);
        let within = Duration::from_millis(body["within_ms"].as_u64().unwrap_or(0));
        match self.find(event, wanted, Instant::now() + within) {
            Some(seen) => {
                self.matched.push(seen);
                Ok(())
            }
            None => Err(format!("no {event} event matching {wanted} within {within:?}")),
        }
    }

    fn expect_no_event(&mut self, body: &Value) -> Result<(), String> {
        let (event, wanted) = (body["event"].as_str().unwrap_or_default(), &body["match"]);
        let period = Duration::from_millis(body["for_ms"].as_u64().unwrap_or(0));
        match self.find(event, wanted, Instant::now() + period) {
            Some(seen) => Err(format!("an unexpected {event} event: {seen}")),
            None => Ok(()),
        }
    }

    fn close(&mut self) -> Result<(), String> {
        self.embed()?.close();
        let closed = self.find("closed", &Value::Null, Instant::now() + CLOSE_WITHIN);
        if closed.is_none() {
            return Err("no closed event after close".into());
        }
        match self.embed()?.poll_event() {
            None => Ok(()),
            Some(after) => Err(format!("an event after closed: {after:?}")),
        }
    }
}
