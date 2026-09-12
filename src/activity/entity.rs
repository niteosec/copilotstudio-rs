use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Metadata object attached to an activity, open by `type`.
///
/// Well-known types have typed views (e.g. [`as_stream_info`](Self::as_stream_info)); everything
/// else is reachable through [`properties`](Self::properties).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Entity {
    /// Entity type (an RFC 3987 IRI or a well-known short name, see
    /// [`constants::entity_types`](super::constants::entity_types)).
    #[serde(rename = "type")]
    pub r#type: String,
    /// Every other property, verbatim.
    #[serde(flatten, default, skip_serializing_if = "Map::is_empty")]
    pub properties: Map<String, Value>,
}

impl Entity {
    /// A property by its wire (camelCase) name.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.properties.get(key)
    }

    /// View this entity as a [`StreamInfo`] (whatever its `type`).
    pub fn as_stream_info(&self) -> Option<StreamInfo> {
        serde_json::from_value(Value::Object(self.properties.clone())).ok()
    }
}

/// The `streaminfo` entity carried by streamed activities.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamInfo {
    /// `streaming` for chunks, `final` on the closing message, `informative` for status text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_type: Option<String>,
    /// 1-based order of this chunk within the stream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_sequence: Option<i64>,
    /// Id shared by every chunk of one streamed response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,
    /// Result of the stream (`success`, `timeout`, `error`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_result: Option<String>,
    /// Whether feedback is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_loop_enabled: Option<bool>,
    /// Feedback-loop payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_loop: Option<Value>,
}
