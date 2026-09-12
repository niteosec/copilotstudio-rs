//! `PowerPlatformCloud` — the Power Platform cloud that hosts an environment.
//!
//! Mirrors `Discovery/PowerPlatformCloud.cs` (.NET), `power_platform_cloud.py`, `powerPlatformCloud.ts`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// The Power Platform cloud you are attempting to connect to.
///
/// The string forms (`"Prod"`, `"GovFR"`, …) are the upstream `EnumMember` values and are what
/// `Display`, `FromStr` (case-insensitive) and serde use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PowerPlatformCloud {
    /// Unknown Power Platform Cloud.
    Unknown,
    /// Internal use only.
    Exp,
    /// Internal use only.
    Dev,
    /// Internal use only.
    Test,
    /// Internal use only.
    Preprod,
    /// Environments created in the First Release cloud.
    FirstRelease,
    /// Environments created in the Production cloud (all geos except First Release and sovereign clouds).
    Prod,
    /// United States GCC cloud.
    Gov,
    /// United States Gov High sovereign cloud.
    High,
    /// United States DoD sovereign cloud.
    DoD,
    /// China sovereign cloud.
    Mooncake,
    /// Restricted sovereign cloud.
    Ex,
    /// Restricted sovereign cloud.
    Rx,
    /// Pull-request validation clusters (internal).
    Prv,
    /// Internal use only.
    Local,
    /// French government sovereign cloud.
    GovFR,
    /// A custom Power Platform API base address (see `ConnectionSettings::custom_power_platform_cloud`).
    Other,
}

impl PowerPlatformCloud {
    /// Every variant, in upstream declaration order.
    pub const ALL: [PowerPlatformCloud; 17] = [
        Self::Unknown,
        Self::Exp,
        Self::Dev,
        Self::Test,
        Self::Preprod,
        Self::FirstRelease,
        Self::Prod,
        Self::Gov,
        Self::High,
        Self::DoD,
        Self::Mooncake,
        Self::Ex,
        Self::Rx,
        Self::Prv,
        Self::Local,
        Self::GovFR,
        Self::Other,
    ];

    /// The upstream string value of this variant.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Exp => "Exp",
            Self::Dev => "Dev",
            Self::Test => "Test",
            Self::Preprod => "Preprod",
            Self::FirstRelease => "FirstRelease",
            Self::Prod => "Prod",
            Self::Gov => "Gov",
            Self::High => "High",
            Self::DoD => "DoD",
            Self::Mooncake => "Mooncake",
            Self::Ex => "Ex",
            Self::Rx => "Rx",
            Self::Prv => "Prv",
            Self::Local => "Local",
            Self::GovFR => "GovFR",
            Self::Other => "Other",
        }
    }
}

impl Default for PowerPlatformCloud {
    /// `Prod`, the default every upstream client applies when no cloud is configured.
    fn default() -> Self {
        Self::Prod
    }
}

impl fmt::Display for PowerPlatformCloud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when a string is not a known cloud name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownCloud(pub String);

impl fmt::Display for UnknownCloud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid PowerPlatformCloud: '{}'", self.0)
    }
}

impl std::error::Error for UnknownCloud {}

impl FromStr for PowerPlatformCloud {
    type Err = UnknownCloud;

    /// Case-insensitive match on the upstream names (`"prod"`, `"PROD"` and `"Prod"` all parse).
    /// Also accepts the Python enum member names (`"FIRST_RELEASE"`, `"GOV_FR"`).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let wanted = s.trim().replace('_', "");
        Self::ALL
            .into_iter()
            .find(|c| c.as_str().eq_ignore_ascii_case(&wanted))
            .ok_or_else(|| UnknownCloud(s.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_case_insensitively_and_python_names() {
        assert_eq!("prod".parse::<PowerPlatformCloud>().unwrap(), PowerPlatformCloud::Prod);
        assert_eq!("GovFR".parse::<PowerPlatformCloud>().unwrap(), PowerPlatformCloud::GovFR);
        assert_eq!("GOV_FR".parse::<PowerPlatformCloud>().unwrap(), PowerPlatformCloud::GovFR);
        assert_eq!("FIRST_RELEASE".parse::<PowerPlatformCloud>().unwrap(), PowerPlatformCloud::FirstRelease);
        assert!("nope".parse::<PowerPlatformCloud>().is_err());
    }

    #[test]
    fn serde_uses_upstream_strings() {
        assert_eq!(serde_json::to_string(&PowerPlatformCloud::DoD).unwrap(), "\"DoD\"");
        assert_eq!(serde_json::from_str::<PowerPlatformCloud>("\"Mooncake\"").unwrap(), PowerPlatformCloud::Mooncake);
    }
}
