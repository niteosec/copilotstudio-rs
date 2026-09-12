use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::ChannelId;

/// Channel account information needed to route a message.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelAccount {
    /// Channel id for the user or agent on this channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Display-friendly name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The account's object id within Entra ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aad_object_id: Option<String>,
    /// Role of the entity behind the account (see [`constants::role_types`](super::constants::role_types)).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Agentic user id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agentic_user_id: Option<String>,
    /// Agentic app id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agentic_app_id: Option<String>,
    /// Tenant id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Any property not modelled above, preserved verbatim.
    #[serde(flatten, default, skip_serializing_if = "Map::is_empty")]
    pub properties: Map<String, Value>,
}

impl ChannelAccount {
    /// An account with only an id.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: Some(id.into()), ..Self::default() }
    }
}

/// The identity of the conversation within a channel.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationAccount {
    /// Whether the conversation has more than two participants.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_group: Option<bool>,
    /// Conversation type, for channels that distinguish them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_type: Option<String>,
    /// The conversation id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Display-friendly name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The account's object id within Entra ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aad_object_id: Option<String>,
    /// Role of the entity behind the account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Tenant id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Conversation properties.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<Value>,
}

impl ConversationAccount {
    /// A conversation account with only an id.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: Some(id.into()), ..Self::default() }
    }
}

/// A reference to a particular point in a conversation.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationReference {
    /// Id of the activity referred to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity_id: Option<String>,
    /// The user participating in the conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<ChannelAccount>,
    /// The agent participating in the conversation. Serialised as `bot`.
    #[serde(rename = "bot", default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<ChannelAccount>,
    /// The conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ConversationAccount>,
    /// Channel id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,
    /// Locale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// Service endpoint for the referenced conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_url: Option<String>,
}
