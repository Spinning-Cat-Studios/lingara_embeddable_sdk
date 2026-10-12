//! A case's `host:` script through the plugin (ADR 1.10.26z D12). The app
//! is `MinimalPlugins` and `LingaraPlugin`; every step is a Message, a
//! component, or (for what the plugin does not wrap) `Lingara::core()`.
//! What the harness sees is the plugin's `LingaraRawEvent`s in K1's JSON,
//! and a refusal (a `LingaraError` no core `Error` event explains) in K1's
//! error vocabulary, ahead of that frame's events as the core's harness
//! puts its synchronous refusals.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use bevy::MinimalPlugins;
use bevy::app::App;
use bevy::ecs::entity::Entity;
use bevy::ecs::message::{Message, Messages};
use bevy_lingara::lingara_embed::{EmbedConfig, EmbedError, EmbedEvent};
use bevy_lingara::{Lingara, LingaraConfig, LingaraDisconnect, LingaraError, LingaraPlugin, LingaraRawEvent, ProvideToken};
use serde_json::{Value, json};

use crate::matching::contains;

/// How long `close` waits for the one `Closed`.
const CLOSE_WITHIN: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct Run {
    pub problems: Vec<String>,
    /// The events the `poll_until` steps matched, in order.
    pub matched: Vec<Value>,
    pub app: Option<App>,
    pub seen: VecDeque<Value>,
    pub last_id: Option<u64>,
    /// K1's ids, to the entities the harness made for them.
    pub entities: HashMap<u64, Entity>,
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
    if let Some(core) = run.app.as_ref().and_then(|a| a.world().get_resource::<Lingara>()).and_then(Lingara::core) {
        core.close_and_wait(1_000);
    }
    run
}

pub fn refusal(error: &EmbedError) -> Value {
    json!({ "type": "error", "error": error })
}

fn take<M: Message>(app: &mut App) -> Vec<M> {
    app.world_mut().resource_mut::<Messages<M>>().drain().collect()
}

/// `now + secs` as RFC 3339, for the `ProvideToken` a start's token becomes.
fn rfc3339_after(secs: u64) -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) + secs;
    let (days, rem) = ((t / 86_400) as i64, t % 86_400);
    // Howard Hinnant's civil-from-days.
    let z = days + 719_468;
    let (era, doe) = (z.div_euclid(146_097), z.rem_euclid(146_097));
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let (d, m) = (doy - (153 * mp + 2) / 5 + 1, if mp < 10 { mp + 3 } else { mp - 9 });
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

impl Run {
    pub fn app(&mut self) -> Result<&mut App, String> {
        self.app.as_mut().ok_or_else(|| "no start step ran".to_owned())
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
        let embed: EmbedConfig = serde_json::from_value(config).map_err(|e| format!("not an EmbedConfig: {e}"))?;
        let mut config = LingaraConfig::new();
        config.embed = embed;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(LingaraPlugin::new(config));
        if let Some(token) = body["initial_token"].as_str().filter(|t| *t != "none") {
            app.world_mut().write_message(ProvideToken::new(token, rfc3339_after(900)));
        }
        self.app = Some(app);
        self.pump()
    }

    /// `expires_at` as a `ProvideToken`; `expires_in_secs` through
    /// `Lingara::core().provide_token_for`, which the plugin does not wrap.
    fn provide(&mut self, body: &Value) -> Result<(), String> {
        let token = body["token"].as_str().unwrap_or_default().to_owned();
        match (body["expires_at"].as_str(), body["expires_in_secs"].as_u64()) {
            (Some(at), _) => {
                self.app()?.world_mut().write_message(ProvideToken::new(token, at));
            }
            (None, Some(secs)) => {
                let result = self.core(|core| core.provide_token_for(token.as_str(), u32::try_from(secs).unwrap_or(u32::MAX)))?;
                if let Err(e) = result {
                    self.seen.push_back(refusal(&e));
                }
            }
            (None, None) => return Err("no expires_at or expires_in_secs".into()),
        }
        self.pump()
    }

    /// `f` over the live core.
    pub fn core<R>(&mut self, f: impl FnOnce(&bevy_lingara::lingara_embed::Embed) -> R) -> Result<R, String> {
        let lingara = self.app()?.world().get_resource::<Lingara>().ok_or("no live handle")?;
        Ok(f(lingara.core().ok_or("no core")?))
    }

    /// One frame. Refusals first, then the frame's raw events.
    pub fn pump(&mut self) -> Result<(), String> {
        let app = self.app()?;
        app.update();
        let raw: Vec<EmbedEvent> = take::<LingaraRawEvent>(app).into_iter().map(|r| r.0).collect();
        let mut errors: Vec<LingaraError> = take::<LingaraError>(app);
        for event in &raw {
            if let EmbedEvent::Error { error } = event
                && let Some(at) = errors.iter().position(|e| &e.error == error)
            {
                errors.remove(at);
            }
        }
        self.seen.extend(errors.iter().map(|e| refusal(&e.error)));
        self.seen.extend(raw.iter().filter_map(|e| serde_json::to_value(e).ok()));
        Ok(())
    }

    /// Frames until an event matches or `deadline` passes.
    fn find(&mut self, event: &str, wanted: &Value, deadline: Instant) -> Result<Option<Value>, String> {
        while Instant::now() < deadline {
            match self.seen.pop_front() {
                Some(seen) if seen["type"] == event && contains(&seen, wanted) => return Ok(Some(seen)),
                Some(_) => {}
                None => {
                    std::thread::sleep(Duration::from_millis(5));
                    self.pump()?;
                }
            }
        }
        Ok(None)
    }

    fn poll_until(&mut self, body: &Value) -> Result<(), String> {
        let (event, wanted) = (body["event"].as_str().unwrap_or_default(), &body["match"]);
        let within = Duration::from_millis(body["within_ms"].as_u64().unwrap_or(0));
        match self.find(event, wanted, Instant::now() + within)? {
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
        match self.find(event, wanted, Instant::now() + period)? {
            Some(seen) => Err(format!("an unexpected {event} event: {seen}")),
            None => Ok(()),
        }
    }

    /// `LingaraDisconnect`, then frames until `Closed`; nothing after it.
    fn close(&mut self) -> Result<(), String> {
        self.app()?.world_mut().write_message(LingaraDisconnect);
        let closed = self.find("closed", &Value::Null, Instant::now() + CLOSE_WITHIN)?;
        if closed.is_none() {
            return Err("no closed event after close".into());
        }
        self.pump()?;
        match self.seen.iter().find(|e| e["type"] != "error") {
            None => Ok(()),
            Some(after) => Err(format!("an event after closed: {after}")),
        }
    }
}
