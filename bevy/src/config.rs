//! `LingaraConfig` (D3): K1's `EmbedConfig`, unchanged, plus the four
//! fields only a Bevy game has. A token never passes through it: the token
//! sources (D5) name where tokens come from, not what they are.

use std::sync::Arc;

use lingara_embed::{EmbedConfig, EmbedTokenSource, Header, TokenSourceConfig};

/// The bindings' cap on one frame's drain (K1's sibling table).
pub const DEFAULT_MAX_EVENTS_PER_FRAME: u32 = 256;

/// What this kit adds to the `User-Agent`, before the game's own suffix.
pub(crate) const KIT_TOKEN: &str = concat!("bevy_lingara/", env!("CARGO_PKG_VERSION"));

#[derive(Clone)]
pub struct LingaraConfig {
    /// K1's config, with K1's names and defaults. Its `user_agent_suffix` is
    /// the game's own, which the plugin sends after `bevy_lingara/<version>`.
    pub embed: EmbedConfig,
    /// At most this many core events become Messages in one frame; the rest
    /// wait for the next, so a burst never spikes a frame.
    pub max_events_per_frame: u32,
    /// `false`: nothing starts until the game sends `LingaraConnect`.
    pub autostart: bool,
    /// The language an `NpcDialogue` that sets none speaks to the player in.
    pub source_lang: Option<String>,
    pub(crate) token_source: Option<Arc<dyn EmbedTokenSource>>,
}

impl Default for LingaraConfig {
    fn default() -> Self {
        Self {
            embed: EmbedConfig::default(),
            max_events_per_frame: DEFAULT_MAX_EVENTS_PER_FRAME,
            autostart: true,
            source_lang: None,
            token_source: None,
        }
    }
}

impl LingaraConfig {
    /// The Poll source: the plugin writes `LingaraTokenNeeded`, and the game
    /// answers with `ProvideToken`.
    pub fn new() -> Self {
        Self::default()
    }

    /// The common case: the core `POST`s to your server's token route with
    /// these headers (your game's own session) and reads `{token, expires_at}`.
    pub fn token_endpoint(mut self, url: impl Into<String>, headers: Vec<Header>) -> Self {
        self.embed.token_source = TokenSourceConfig::Http { url: url.into(), headers };
        self
    }

    /// Your own async token code, awaited on the core's thread, never Bevy's.
    pub fn token_source(mut self, source: Arc<dyn EmbedTokenSource>) -> Self {
        self.token_source = Some(source);
        self
    }

    pub fn source_lang(mut self, lang: impl Into<String>) -> Self {
        self.source_lang = Some(lang.into());
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.embed.base_url = url.into();
        self
    }

    pub fn autostart(mut self, autostart: bool) -> Self {
        self.autostart = autostart;
        self
    }

    pub fn max_events_per_frame(mut self, max: u32) -> Self {
        self.max_events_per_frame = max;
        self
    }

    /// Your game's own token, e.g. `my-game/1.4`.
    pub fn user_agent_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.embed.user_agent_suffix = Some(suffix.into());
        self
    }

    /// `false` starts the stream paused: a game that reads no stream event pays nothing.
    pub fn stream_on_start(mut self, on: bool) -> Self {
        self.embed.stream_on_start = on;
        self
    }

    /// A cursor kept from `LingaraStatus`, to resume the stream with no gap.
    pub fn resume_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.embed.resume_cursor = Some(cursor.into());
        self
    }

    /// The config the core starts with: the kit names itself first (EK10).
    pub(crate) fn core_config(&self) -> EmbedConfig {
        let mut config = self.embed.clone();
        config.user_agent_suffix = Some(user_agent(config.user_agent_suffix.as_deref()));
        config
    }
}

/// `bevy_lingara/<version>`, then the game's suffix when it sets one.
pub(crate) fn user_agent(game: Option<&str>) -> String {
    match game.map(str::trim).filter(|s| !s.is_empty()) {
        Some(game) => format!("{KIT_TOKEN} {game}"),
        None => KIT_TOKEN.to_owned(),
    }
}

impl std::fmt::Debug for LingaraConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LingaraConfig")
            .field("embed", &self.embed)
            .field("max_events_per_frame", &self.max_events_per_frame)
            .field("autostart", &self.autostart)
            .field("source_lang", &self.source_lang)
            .field("token_source", &self.token_source.is_some())
            .finish()
    }
}
