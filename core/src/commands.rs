//! `execute(Command)` and the token and stream commands (D3, D4, D5). Every
//! flat method is the same call as its `Command` variant, so a binding that
//! moves JSON deserialises one enum and dispatches it here.

use crate::embed::Embed;
use crate::error::{EmbedError, ErrorKind};
use crate::secret::Secret;
use crate::token_fetch::http_mode;
use crate::types::{Command, CommandOutcome, Header};

/// `Ok` with the command's result when a group handles it, `Err` handing
/// the command on to the next group.
pub(crate) type Dispatch = Result<Result<CommandOutcome, EmbedError>, Command>;

fn done(result: Result<(), EmbedError>) -> Result<CommandOutcome, EmbedError> {
    result.map(|()| CommandOutcome::Done {})
}

fn id(result: Result<u64, EmbedError>) -> Result<CommandOutcome, EmbedError> {
    result.map(|id| CommandOutcome::Id { id })
}

impl Embed {
    /// Dispatches one command: the flat method it names, exactly.
    pub fn execute(&self, command: Command) -> Result<CommandOutcome, EmbedError> {
        let dispatched = self.exec_token(command).or_else(|c| self.exec_practice(c)).or_else(|c| self.exec_dialogue(c));
        dispatched.unwrap_or_else(|unhandled| Err(EmbedError::new(ErrorKind::Internal, format!("no dispatch for {unhandled:?}"))))
    }

    /// Token, stream and world commands.
    fn exec_token(&self, command: Command) -> Dispatch {
        Ok(match command {
            Command::ProvideToken { token, expires_at } => done(self.provide_token(token, expires_at)),
            Command::ProvideTokenFor { token, expires_in_secs } => done(self.provide_token_for(token, expires_in_secs)),
            Command::TokenUnavailable { reason } => done(self.token_unavailable(&reason)),
            Command::SetTokenEndpoint { url, headers } => done(self.set_token_endpoint(&url, headers)),
            Command::StreamPause => done(self.stream_pause()),
            Command::StreamResume => done(self.stream_resume()),
            Command::SendContext { context } => id(self.send_context(context)),
            other => return Err(other),
        })
    }

    /// The host's token, valid until `expires_at` (RFC 3339) on its clock.
    pub fn provide_token(&self, token: impl Into<Secret>, expires_at: impl Into<String>) -> Result<(), EmbedError> {
        self.shared.live()?;
        self.shared.tokens.store(&crate::handoff::IssuedToken { token: token.into(), expires_at: Some(expires_at.into()), expires_in_secs: None })
    }

    /// The host's token with its lifetime from the mint's `expires_in`: for
    /// a host that holds the whole mint answer and distrusts its clock.
    pub fn provide_token_for(&self, token: impl Into<Secret>, expires_in_secs: u32) -> Result<(), EmbedError> {
        self.shared.live()?;
        self.shared.tokens.store(&crate::handoff::IssuedToken { token: token.into(), expires_at: None, expires_in_secs: Some(expires_in_secs) })
    }

    /// The host cannot get a token: every waiting call fails `TokenUnavailable`.
    pub fn token_unavailable(&self, reason: &str) -> Result<(), EmbedError> {
        self.shared.live()?;
        self.shared.tokens.fail_all(EmbedError::new(ErrorKind::TokenUnavailable, format!("the host has no token: {reason}")));
        Ok(())
    }

    /// Switches to, or re-points, the Http token source: for a player who
    /// signs in to the game mid-session.
    pub fn set_token_endpoint(&self, url: &str, headers: Vec<Header>) -> Result<(), EmbedError> {
        self.shared.live()?;
        self.shared.tokens.set_mode(http_mode(url, &headers)?);
        Ok(())
    }

    /// Drops the stream's connection, keeping its cursor.
    pub fn stream_pause(&self) -> Result<(), EmbedError> {
        self.shared.live()?;
        self.shared.stream.pause(&self.shared);
        Ok(())
    }

    /// Reopens the stream from the kept cursor.
    pub fn stream_resume(&self) -> Result<(), EmbedError> {
        self.shared.live()?;
        self.shared.stream.resume(&self.shared);
        Ok(())
    }
}
