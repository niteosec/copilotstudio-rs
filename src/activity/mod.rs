//! The Bot Framework **Activity** schema, as used on the Direct-to-Engine wire.
//!
//! Mirrors `microsoft_agents.activity.Activity` (Python) and `Microsoft.Agents.Core.Models.Activity`
//! (.NET) at the pinned upstream. Wire rules (see `docs/DESIGN.md` §3.8): camelCase names, absent
//! fields omitted, `from_property` ↔ `from`, unknown top-level properties preserved in
//! [`Activity::properties`], entities open by `type`.
//!
//! Timestamps are kept as the RFC 3339 strings the service sends (`docs/DESIGN.md` D5).

mod accounts;
mod attachments;
mod channel_id;
pub mod constants;
mod entity;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub use accounts::{ChannelAccount, ConversationAccount, ConversationReference};
pub use attachments::{Attachment, CardAction, SuggestedActions};
pub use channel_id::ChannelId;
pub use entity::{Entity, StreamInfo};

/// The activity type. Well-known values are variants; anything else is [`Other`](ActivityType::Other).
///
/// Serialises to the upstream strings (`"message"`, `"conversationUpdate"`, …).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActivityType {
    /// `message`
    Message,
    /// `contactRelationUpdate`
    ContactRelationUpdate,
    /// `conversationUpdate`
    ConversationUpdate,
    /// `typing`
    Typing,
    /// `endOfConversation`
    EndOfConversation,
    /// `event`
    Event,
    /// `invoke`
    Invoke,
    /// `invokeResponse`
    InvokeResponse,
    /// `deleteUserData`
    DeleteUserData,
    /// `messageUpdate`
    MessageUpdate,
    /// `messageDelete`
    MessageDelete,
    /// `installationUpdate`
    InstallationUpdate,
    /// `messageReaction`
    MessageReaction,
    /// `suggestion`
    Suggestion,
    /// `trace`
    Trace,
    /// `handoff`
    Handoff,
    /// `command`
    Command,
    /// `commandResult`
    CommandResult,
    /// Any other type string.
    Other(String),
}

impl ActivityType {
    /// The wire string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Message => "message",
            Self::ContactRelationUpdate => "contactRelationUpdate",
            Self::ConversationUpdate => "conversationUpdate",
            Self::Typing => "typing",
            Self::EndOfConversation => "endOfConversation",
            Self::Event => "event",
            Self::Invoke => "invoke",
            Self::InvokeResponse => "invokeResponse",
            Self::DeleteUserData => "deleteUserData",
            Self::MessageUpdate => "messageUpdate",
            Self::MessageDelete => "messageDelete",
            Self::InstallationUpdate => "installationUpdate",
            Self::MessageReaction => "messageReaction",
            Self::Suggestion => "suggestion",
            Self::Trace => "trace",
            Self::Handoff => "handoff",
            Self::Command => "command",
            Self::CommandResult => "commandResult",
            Self::Other(s) => s,
        }
    }
}

impl Default for ActivityType {
    /// `Message` — so `Activity::default()` is an empty message.
    fn default() -> Self {
        Self::Message
    }
}

impl From<&str> for ActivityType {
    fn from(s: &str) -> Self {
        match s {
            "message" => Self::Message,
            "contactRelationUpdate" => Self::ContactRelationUpdate,
            "conversationUpdate" => Self::ConversationUpdate,
            "typing" => Self::Typing,
            "endOfConversation" => Self::EndOfConversation,
            "event" => Self::Event,
            "invoke" => Self::Invoke,
            "invokeResponse" => Self::InvokeResponse,
            "deleteUserData" => Self::DeleteUserData,
            "messageUpdate" => Self::MessageUpdate,
            "messageDelete" => Self::MessageDelete,
            "installationUpdate" => Self::InstallationUpdate,
            "messageReaction" => Self::MessageReaction,
            "suggestion" => Self::Suggestion,
            "trace" => Self::Trace,
            "handoff" => Self::Handoff,
            "command" => Self::Command,
            "commandResult" => Self::CommandResult,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl From<String> for ActivityType {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl std::fmt::Display for ActivityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for ActivityType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ActivityType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::from(String::deserialize(deserializer)?))
    }
}

/// An Activity is the basic communication type of the protocol.
///
/// Every field but `type` is optional and omitted from JSON when `None`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    /// The activity type.
    #[serde(rename = "type")]
    pub r#type: ActivityType,
    /// Uniquely identifies the activity on the channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// UTC send time, ISO-8601 / RFC 3339.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    /// Local send time, ISO-8601 with offset (e.g. `2016-09-23T13:07:49.4714686-07:00`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_timestamp: Option<String>,
    /// IANA time-zone name of the local timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_timezone: Option<String>,
    /// The channel's service endpoint; set by the channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_url: Option<String>,
    /// The channel (and optional sub-channel) id; set by the channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,
    /// The sender. Serialised as `from`.
    #[serde(rename = "from", default, skip_serializing_if = "Option::is_none")]
    pub from_property: Option<ChannelAccount>,
    /// The conversation the activity belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ConversationAccount>,
    /// The recipient.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient: Option<ChannelAccount>,
    /// Format of `text` (see [`constants::text_format_types`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_format: Option<String>,
    /// Layout hint for multiple attachments (see [`constants::attachment_layout_types`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_layout: Option<String>,
    /// Members added to the conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members_added: Option<Vec<ChannelAccount>>,
    /// Members removed from the conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members_removed: Option<Vec<ChannelAccount>>,
    /// Reactions added.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reactions_added: Option<Vec<MessageReaction>>,
    /// Reactions removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reactions_removed: Option<Vec<MessageReaction>>,
    /// The updated topic name of the conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic_name: Option<String>,
    /// Whether prior history of the channel is disclosed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_disclosed: Option<bool>,
    /// BCP-47 locale of `text`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// The text content of the message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The text to speak.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speak: Option<String>,
    /// Input hint (see [`constants::input_hints`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_hint: Option<String>,
    /// Text to display if the channel cannot render cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Suggested actions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_actions: Option<SuggestedActions>,
    /// Attachments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Attachment>>,
    /// Entities mentioned in or attached to the activity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entities: Option<Vec<Entity>>,
    /// Channel-specific content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_data: Option<Value>,
    /// For `contactRelationUpdate`: `add` / `remove`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// The id of the message this is a reply to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to_id: Option<String>,
    /// A descriptive label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The type of `value`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    /// A value associated with the activity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// The operation name of an `invoke` or `event` activity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A reference to another conversation or activity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relates_to: Option<ConversationReference>,
    /// For `endOfConversation`: why it ended (see [`constants::end_of_conversation_codes`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// When the activity should be considered expired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiration: Option<String>,
    /// Importance (see [`constants::activity_importance`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importance: Option<String>,
    /// Delivery mode (see [`constants::delivery_modes`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_mode: Option<String>,
    /// Phrases speech/language priming systems should listen for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen_for: Option<Vec<String>>,
    /// Text fragments to highlight when `reply_to_id` is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_highlights: Option<Vec<TextHighlight>>,
    /// A programmatic action accompanying the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_action: Option<SemanticAction>,
    /// An IRI identifying the caller; populated by agents/clients, not meant for the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caller_id: Option<String>,
    /// Any property not modelled above, preserved verbatim.
    #[serde(flatten, default, skip_serializing_if = "Map::is_empty")]
    pub properties: Map<String, Value>,
}

impl Activity {
    /// An empty activity of the given type.
    pub fn new(r#type: impl Into<ActivityType>) -> Self {
        Self { r#type: r#type.into(), ..Self::default() }
    }

    /// A `message` activity with `text`.
    pub fn message(text: impl Into<String>) -> Self {
        Self { r#type: ActivityType::Message, text: Some(text.into()), ..Self::default() }
    }

    /// Set the conversation id, creating the conversation account if needed.
    pub fn with_conversation_id(mut self, id: impl Into<String>) -> Self {
        self.set_conversation_id(id);
        self
    }

    /// Set the conversation id in place, creating the conversation account if needed.
    pub fn set_conversation_id(&mut self, id: impl Into<String>) {
        match &mut self.conversation {
            Some(c) => c.id = Some(id.into()),
            None => self.conversation = Some(ConversationAccount::new(id)),
        }
    }

    /// `conversation.id`, when present and non-empty.
    pub fn conversation_id(&self) -> Option<&str> {
        self.conversation.as_ref().and_then(|c| c.id.as_deref()).filter(|s| !s.is_empty())
    }

    /// The `streaminfo` entity of a streamed chunk, or its legacy `channelData` equivalent.
    ///
    /// Present on `typing` activities that carry incremental text, and on the final `message`
    /// (`stream_type: "final"`). See `docs/DESIGN.md` §3.7.
    pub fn stream_info(&self) -> Option<StreamInfo> {
        if let Some(e) =
            self.entities.iter().flatten().find(|e| e.r#type.eq_ignore_ascii_case(constants::entity_types::STREAM_INFO))
        {
            return e.as_stream_info();
        }
        let data = self.channel_data.as_ref()?.as_object()?;
        if data.get("streamType").is_some() || data.get("streamId").is_some() {
            return serde_json::from_value(Value::Object(data.clone())).ok();
        }
        None
    }
}

/// A message reaction.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageReaction {
    /// Reaction type (e.g. `like`, `plusOne`).
    #[serde(rename = "type")]
    pub r#type: String,
}

/// A substring of content within another field.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextHighlight {
    /// The snippet of text to highlight.
    pub text: String,
    /// Occurrence of the text within the referenced text, if multiple exist.
    pub occurrence: i64,
}

/// A reference to a programmatic action.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticAction {
    /// Id of this action.
    pub id: String,
    /// Entities associated with this action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entities: Option<Map<String, Value>>,
    /// State: `start`, `continue`, `done`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}
