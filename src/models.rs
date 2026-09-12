//! Request and response bodies of the Direct-to-Engine protocol.
//!
//! Mirrors `Models/*.cs` (.NET), `start_request.py` / `execute_turn_request.py` /
//! `subscribe_event.py`, `startRequest.ts` / `executeTurnRequest.ts` / `responses.ts`.

use serde::{Deserialize, Serialize};

use crate::activity::Activity;

/// Body of `POST …/conversations` — starts a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRequest {
    /// The locale to use as defined by the client (e.g. `en-US`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// Whether the agent should emit its conversation-start event (greeting topic). Default `true`.
    pub emit_start_conversation_event: bool,
    /// A conversation id requested by the client; the service generates one when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

impl Default for StartRequest {
    fn default() -> Self {
        Self { locale: None, emit_start_conversation_event: true, conversation_id: None }
    }
}

impl StartRequest {
    /// `StartRequest` with only the start-event flag set (the `start_conversation(bool)` shape).
    pub fn new(emit_start_conversation_event: bool) -> Self {
        Self { emit_start_conversation_event, ..Self::default() }
    }
}

/// Body of `POST …/conversations/{id}` — executes one turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecuteTurnRequest {
    /// The activity sent as the authenticated user.
    pub activity: Activity,
}

/// JSON-fallback body of a start request (returned when the service does not stream).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartResponse {
    /// The activities that should be shown on the client.
    #[serde(default)]
    pub activities: Vec<Activity>,
    /// The id of the conversation that was started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

/// JSON-fallback body of an execute-turn request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExecuteTurnResponse {
    /// The activities that should be shown on the client.
    #[serde(default)]
    pub activities: Vec<Activity>,
}

/// JSON-fallback body of a subscribe request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SubscribeResponse {
    /// The activities that should be shown on the client.
    #[serde(default)]
    pub activities: Vec<Activity>,
}

/// One event from [`CopilotClient::subscribe`](crate::CopilotClient::subscribe).
#[derive(Debug, Clone, PartialEq)]
pub struct SubscribeEvent {
    /// The activity received from the agent.
    pub activity: Activity,
    /// The SSE event id for resumption (`None` for JSON-fallback responses).
    pub event_id: Option<String>,
}
