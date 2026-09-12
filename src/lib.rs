//! Unofficial Rust port of Microsoft's Copilot Studio client (Direct-to-Engine protocol) from the
//! open-source Microsoft 365 Agents SDK (`microsoft/Agents`).
//!
//! The port mirrors the official client's public surface at a pinned upstream version — see
//! [`UPSTREAM`] and `docs/DESIGN.md` — and keeps token acquisition outside the crate: you supply a
//! delegated (or on-behalf-of) Entra token for the audience returned by [`scope_from_settings`].
//!
//! ```no_run
//! use copilotstudio_client::{ConnectionSettings, CopilotClient};
//! use futures_util::TryStreamExt;
//!
//! # async fn run() -> Result<(), copilotstudio_client::Error> {
//! let settings = ConnectionSettings::new("<environment id>", "<agent schema name>");
//! // Ask Entra for a token with this audience (delegated `CopilotStudio.Copilots.Invoke`):
//! let _scope = copilotstudio_client::scope_from_settings(&settings)?;
//! let client = CopilotClient::new(settings, "<access token>");
//!
//! let mut start = client.start_conversation(true);
//! while let Some(activity) = start.try_next().await? {
//!     println!("{}: {}", activity.r#type, activity.text.as_deref().unwrap_or(""));
//! }
//! let mut reply = client.ask_question("What can you do?", None);
//! while let Some(activity) = reply.try_next().await? {
//!     if let Some(text) = activity.text.as_deref() {
//!         println!("agent> {text}");
//!     }
//! }
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

pub mod activity;
mod agent_type;
mod client;
mod connection_settings;
mod error;
pub mod headers;
mod models;
pub mod power_platform_cloud;
pub mod power_platform_environment;
pub mod sse;
mod token;
mod user_agent;

pub use agent_type::{AgentType, UnknownAgentType};
pub use client::{ActivityStream, CopilotClient, CopilotClientBuilder, SubscribeStream};
pub use connection_settings::ConnectionSettings;
pub use error::{BoxError, Error, SettingsError};
pub use models::{
    ExecuteTurnRequest, ExecuteTurnResponse, StartRequest, StartResponse, SubscribeEvent, SubscribeResponse,
};
pub use power_platform_cloud::{PowerPlatformCloud, UnknownCloud};
pub use power_platform_environment::{
    API_VERSION, connection_url, scope_from_cloud, scope_from_settings, subscribe_url,
};
pub use token::{BoxFuture, StaticToken, TokenFn, TokenProvider};
pub use user_agent::user_agent;

// Re-exported for convenience: the schema types most callers touch.
pub use activity::{Activity, ActivityType};

/// The upstream this crate mirrors (see `UPSTREAM.toml` and `docs/DESIGN.md` §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpstreamPin {
    /// `microsoft/Agents-for-net` tag (primary reference).
    pub net_tag: &'static str,
    /// `microsoft/Agents-for-net` commit.
    pub net_commit: &'static str,
    /// `microsoft/Agents-for-python` tag.
    pub python_tag: &'static str,
    /// `microsoft/Agents-for-python` commit.
    pub python_commit: &'static str,
    /// `microsoft/Agents-for-js` tag.
    pub js_tag: &'static str,
    /// `microsoft/Agents-for-js` commit.
    pub js_commit: &'static str,
}

/// The pinned upstream versions.
pub const UPSTREAM: UpstreamPin = UpstreamPin {
    net_tag: "v1.8.77",
    net_commit: "d2493a5feebaaf742a456d24f8caa9878d3f2373",
    python_tag: "v1.5.0",
    python_commit: "d3f427d81b6d5fdd40bcb8ad16b78dbeb4530631",
    js_tag: "v1.8",
    js_commit: "8f07a596a54b59219435af4a574d0740c814d36b",
};
