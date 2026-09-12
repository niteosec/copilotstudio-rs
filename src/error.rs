//! Error types.
//!
//! `SettingsError` messages are the upstream messages verbatim (`errors/error_resources.py`,
//! `errorHelper.ts`, the `ArgumentException`s in `PowerPlatformEnvironment.cs`) so logs line up
//! across SDKs. Numeric codes are not carried: Python and JS assign different ones.

use http::StatusCode;

use crate::power_platform_cloud::PowerPlatformCloud;

/// A boxed error from a caller-supplied component (the [`TokenProvider`](crate::TokenProvider)).
pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Errors raised while resolving a connection URL or token audience from [`ConnectionSettings`](crate::ConnectionSettings).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    /// `PowerPlatformCloud::Other` without a base address.
    #[error("cloud_base_address must be provided when PowerPlatformCloud is Other")]
    CloudBaseAddressRequired,
    /// Standard mode without an environment id.
    #[error("EnvironmentId must be provided")]
    EnvironmentIdRequired,
    /// Standard mode without a schema name.
    #[error("AgentIdentifier must be provided")]
    AgentIdentifierRequired,
    /// `PowerPlatformCloud::Other` with neither a usable base address nor a custom cloud.
    #[error("Either CustomPowerPlatformCloud or cloud_base_address must be provided when PowerPlatformCloud is Other")]
    CustomCloudOrBaseAddressRequired,
    /// Audience requested with neither settings nor a cloud.
    #[error("Either settings or cloud must be provided")]
    SettingsOrCloudRequired,
    /// A direct-connect URL that is not absolute.
    #[error("DirectConnectUrl is invalid")]
    InvalidDirectConnectUrl,
    /// A direct-connect URL on an unrecognised host and no usable cloud to fall back on.
    #[error(
        "Unable to resolve the PowerPlatform Cloud from DirectConnectUrl. The Token Audience resolver requires a specific PowerPlatformCloudCategory."
    )]
    UnableToResolveCloudFromDirectConnectUrl,
    /// A cloud with no endpoint suffix (`Unknown`).
    #[error("Invalid cloud category value: {0}")]
    InvalidCloudCategory(PowerPlatformCloud),
    /// An environment id too short to split into the host prefix/suffix.
    #[error("EnvironmentId is invalid")]
    InvalidEnvironmentId,
    /// A custom cloud / base address that cannot form a host.
    #[error("customPowerPlatformCloud must be a valid URL")]
    InvalidCustomPowerPlatformCloud,
    /// `ConnectionSettings::from_env` found a value it cannot parse.
    #[error("environment variable {name} has an invalid value: {value:?}")]
    InvalidEnvironmentVariable {
        /// The variable name.
        name: &'static str,
        /// The offending value.
        value: String,
    },
}

/// Errors raised by [`CopilotClient`](crate::CopilotClient).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The connection settings cannot produce a URL or audience.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// A required argument was empty (`conversation_id` on `execute` / `subscribe`).
    #[error("{0}")]
    InvalidArgument(&'static str),
    /// The [`TokenProvider`](crate::TokenProvider) failed.
    #[error("token provider failed: {0}")]
    Token(#[source] BoxError),
    /// Transport-level failure (connect, TLS, body read).
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    /// The service answered with a non-success status.
    #[error("Error sending request: {status}. {body}")]
    RequestFailed {
        /// The HTTP status.
        status: StatusCode,
        /// The response body, as text (may be empty).
        body: String,
    },
    /// A response payload could not be decoded.
    #[error("failed to decode {context}: {source}")]
    Decode {
        /// What was being decoded (`"activity"`, `"StartResponse"`, …).
        context: &'static str,
        /// The underlying serde error.
        #[source]
        source: serde_json::Error,
    },
}

impl Error {
    pub(crate) fn decode(context: &'static str, source: serde_json::Error) -> Self {
        Self::Decode { context, source }
    }
}
