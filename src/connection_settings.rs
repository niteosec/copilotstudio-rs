//! `ConnectionSettings` — how to reach one Copilot Studio agent.
//!
//! Mirrors `ConnectionSettings.cs` / `ICopilotStudioClientConnectionSettings.cs` (.NET),
//! `connection_settings.py`, `connectionSettings.ts`. Validation happens when a URL or audience is
//! resolved (see [`power_platform_environment`](crate::power_platform_environment)), as in .NET.

use crate::agent_type::AgentType;
use crate::error::SettingsError;
use crate::power_platform_cloud::PowerPlatformCloud;

/// Configuration for the Direct-to-Engine client.
///
/// Either `direct_connect_url` **or** the pair `environment_id` + `schema_name` must be set. When
/// `direct_connect_url` is set every other addressing field is ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectionSettings {
    /// Environment id of the Power Platform environment hosting the agent
    /// (Copilot Studio → Settings → Advanced → Metadata).
    pub environment_id: Option<String>,
    /// Schema name of the agent (same metadata page). Called `agent_identifier` in the Python client.
    pub schema_name: Option<String>,
    /// Power Platform cloud hosting the environment. `None` / `Unknown` resolve to `Prod`.
    pub cloud: Option<PowerPlatformCloud>,
    /// Type of agent. `None` resolves to `Published`.
    pub copilot_agent_type: Option<AgentType>,
    /// When `cloud` is `Other`, the Power Platform API base address (e.g. `api.contoso.example`).
    pub custom_power_platform_cloud: Option<String>,
    /// Absolute URL to connect directly to a Copilot Studio endpoint. When set, all other
    /// addressing settings are ignored.
    pub direct_connect_url: Option<String>,
    /// Ask the service for its island-specific experimental endpoint and switch to it once offered.
    pub use_experimental_endpoint: bool,
    /// Log request URLs and response headers at `debug` level (bearer tokens are never logged).
    pub enable_diagnostics: bool,
}

impl ConnectionSettings {
    /// Settings for a published agent in `Prod`, addressed by environment id and schema name.
    pub fn new(environment_id: impl Into<String>, schema_name: impl Into<String>) -> Self {
        Self { environment_id: Some(environment_id.into()), schema_name: Some(schema_name.into()), ..Self::default() }
    }

    /// Settings that connect directly to `url`, ignoring every other addressing field.
    pub fn direct(url: impl Into<String>) -> Self {
        Self { direct_connect_url: Some(url.into()), ..Self::default() }
    }

    /// Set the cloud (default `Prod`).
    pub fn cloud(mut self, cloud: PowerPlatformCloud) -> Self {
        self.cloud = Some(cloud);
        self
    }

    /// Set the agent type (default `Published`).
    pub fn agent_type(mut self, agent_type: AgentType) -> Self {
        self.copilot_agent_type = Some(agent_type);
        self
    }

    /// Set the custom Power Platform API base address used with `PowerPlatformCloud::Other`.
    pub fn custom_power_platform_cloud(mut self, base_address: impl Into<String>) -> Self {
        self.custom_power_platform_cloud = Some(base_address.into());
        self
    }

    /// Set a direct-connect URL, which overrides every other addressing field.
    pub fn direct_connect_url(mut self, url: impl Into<String>) -> Self {
        self.direct_connect_url = Some(url.into());
        self
    }

    /// Opt into the island experimental endpoint (default `false`).
    pub fn use_experimental_endpoint(mut self, enabled: bool) -> Self {
        self.use_experimental_endpoint = enabled;
        self
    }

    /// Enable diagnostic logging (default `false`).
    pub fn enable_diagnostics(mut self, enabled: bool) -> Self {
        self.enable_diagnostics = enabled;
        self
    }

    /// Build settings from environment variables, using the Python client's names:
    ///
    /// | Variable | Field |
    /// |---|---|
    /// | `ENVIRONMENT_ID` | `environment_id` |
    /// | `SCHEMA_NAME`, else `AGENT_IDENTIFIER` | `schema_name` |
    /// | `CLOUD` | `cloud` (case-insensitive name; default `Prod`) |
    /// | `COPILOT_AGENT_TYPE` | `copilot_agent_type` (default `Published`) |
    /// | `CUSTOM_POWER_PLATFORM_CLOUD` | `custom_power_platform_cloud` |
    /// | `DIRECT_CONNECT_URL` | `direct_connect_url` |
    /// | `USE_EXPERIMENTAL_ENDPOINT` | `use_experimental_endpoint` (`"true"`, case-insensitive) |
    /// | `ENABLE_DIAGNOSTICS` | `enable_diagnostics` (`"true"`, case-insensitive) |
    ///
    /// Divergence: an unparseable `CLOUD` / `COPILOT_AGENT_TYPE` is an error here; the Python
    /// client silently falls back to the default.
    pub fn from_env() -> Result<Self, SettingsError> {
        fn var(name: &str) -> Option<String> {
            std::env::var(name).ok().map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
        }
        fn flag(name: &str) -> bool {
            var(name).is_some_and(|v| v.eq_ignore_ascii_case("true"))
        }

        let cloud = match var("CLOUD") {
            Some(v) => {
                Some(v.parse().map_err(|_| SettingsError::InvalidEnvironmentVariable { name: "CLOUD", value: v })?)
            }
            None => None,
        };
        let copilot_agent_type = match var("COPILOT_AGENT_TYPE") {
            Some(v) => Some(
                v.parse()
                    .map_err(|_| SettingsError::InvalidEnvironmentVariable { name: "COPILOT_AGENT_TYPE", value: v })?,
            ),
            None => None,
        };

        Ok(Self {
            environment_id: var("ENVIRONMENT_ID"),
            schema_name: var("SCHEMA_NAME").or_else(|| var("AGENT_IDENTIFIER")),
            cloud,
            copilot_agent_type,
            custom_power_platform_cloud: var("CUSTOM_POWER_PLATFORM_CLOUD"),
            direct_connect_url: var("DIRECT_CONNECT_URL"),
            use_experimental_endpoint: flag("USE_EXPERIMENTAL_ENDPOINT"),
            enable_diagnostics: flag("ENABLE_DIAGNOSTICS"),
        })
    }
}
