use serde::{Deserialize, Serialize};

/// A channel id in the form `channel[:subChannel]`, serialised as a plain string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelId(pub String);

impl ChannelId {
    /// Build from a channel and optional sub-channel.
    pub fn new(channel: impl Into<String>, sub_channel: Option<&str>) -> Self {
        let channel = channel.into();
        match sub_channel.map(str::trim).filter(|s| !s.is_empty()) {
            Some(sub) => Self(format!("{channel}:{sub}")),
            None => Self(channel),
        }
    }

    /// The main channel (`email` in `email:work`).
    pub fn channel(&self) -> &str {
        self.0.split_once(':').map_or(self.0.as_str(), |(c, _)| c).trim()
    }

    /// The sub-channel (`work` in `email:work`), if any.
    pub fn sub_channel(&self) -> Option<&str> {
        self.0.split_once(':').map(|(_, s)| s.trim()).filter(|s| !s.is_empty())
    }
}

impl From<&str> for ChannelId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl From<String> for ChannelId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
