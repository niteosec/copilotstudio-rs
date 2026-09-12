use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An attachment within an activity.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    /// MIME type of the attachment (e.g. `application/vnd.microsoft.card.adaptive`).
    pub content_type: String,
    /// Content URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_url: Option<String>,
    /// Embedded content (a card, for instance).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,
    /// Name of the attachment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Thumbnail URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
}

/// Actions suggested to the user.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedActions {
    /// Ids of the recipients the actions should be shown to. `None` when absent; an explicitly
    /// empty list is preserved as such.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<String>>,
    /// The actions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<CardAction>>,
}

/// A clickable action.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardAction {
    /// Action type (see [`constants::action_types`](super::constants::action_types)).
    #[serde(rename = "type")]
    pub r#type: String,
    /// Button text.
    pub title: String,
    /// Image URL shown next to the text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Text for this action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Text shown in the chat feed when clicked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_text: Option<String>,
    /// Action payload; depends on the type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// Channel-specific data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_data: Option<Value>,
    /// Alternate text for `image`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_alt_text: Option<String>,
}
