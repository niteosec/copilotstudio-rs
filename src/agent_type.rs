//! `AgentType` — the kind of agent addressed in Copilot Studio.
//!
//! Mirrors `Discovery/AgentType.cs` (.NET), `agent_type.py`, `agentType.ts`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Agent types that can be connected to by the Copilot Studio client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AgentType {
    /// A Copilot Studio published agent (the default). Addressed under `dataverse-backed`.
    #[default]
    Published,
    /// A system pre-built agent. Addressed under `prebuilt`.
    Prebuilt,
}

impl AgentType {
    /// The upstream string value (`"Published"` / `"Prebuilt"`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Published => "Published",
            Self::Prebuilt => "Prebuilt",
        }
    }

    /// The path segment used in the D2E URL (`dataverse-backed` / `prebuilt`).
    pub const fn path_segment(self) -> &'static str {
        match self {
            Self::Published => "dataverse-backed",
            Self::Prebuilt => "prebuilt",
        }
    }
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when a string is not a known agent type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownAgentType(pub String);

impl fmt::Display for UnknownAgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid AgentType: '{}'. Supported values: Published, Prebuilt", self.0)
    }
}

impl std::error::Error for UnknownAgentType {}

impl FromStr for AgentType {
    type Err = UnknownAgentType;

    /// Case-insensitive (`"published"`, `"PUBLISHED"`, `"Published"`).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("published") {
            Ok(Self::Published)
        } else if s.eq_ignore_ascii_case("prebuilt") {
            Ok(Self::Prebuilt)
        } else {
            Err(UnknownAgentType(s.to_owned()))
        }
    }
}
