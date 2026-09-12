//! Header names used on the Direct-to-Engine wire.
//!
//! Mirrors `CopilotStudioHeaderNames.cs` (.NET). Only the first four are used by the client at the
//! pinned upstream; the rest are exposed for parity.

/// Request header carrying the conversation id when a `StartRequest` names one (.NET).
pub const CONVERSATION_ID: &str = "x-ms-conversation-id";
/// Response header: the conversation id assigned by the service.
pub const D2E_CONVERSATION_ID: &str = "x-ms-conversationid";
/// Response header: the island-specific experimental endpoint (see `use_experimental_endpoint`).
pub const D2E_EXPERIMENTAL_URL: &str = "x-ms-d2e-experimental";
/// Request header for SSE resumption on `subscribe`.
pub const LAST_EVENT_ID: &str = "last-event-id";

/// Client request correlation id (defined upstream, not sent at the pinned version).
pub const CLIENT_REQUEST_ID: &str = "x-ms-client-request-id";
/// End-to-end correlation id (defined upstream, not sent at the pinned version).
pub const CORRELATION_ID: &str = "x-ms-correlation-id";
/// Agent version (defined upstream, not sent at the pinned version).
pub const AGENT_VERSION: &str = "x-cci-agent-version";

/// `Accept` value that asks the service for a streamed response.
pub const EVENT_STREAM: &str = "text/event-stream";
/// `Content-Type` of every request body.
pub const APPLICATION_JSON: &str = "application/json";
