//! The client contract as a trait — `ICopilotClient` (.NET) / `CopilotClientProtocol` (Python).
//!
//! [`CopilotClient`] implements it; consumers that need to substitute a test double hold an
//! `Arc<dyn CopilotClientApi>` (or a generic `C: CopilotClientApi`) instead of the concrete type.

use std::sync::Arc;

use crate::activity::Activity;
use crate::client::{ActivityStream, CopilotClient, SubscribeStream};
use crate::models::StartRequest;

/// Contract for a client that connects to the Direct-to-Engine endpoint of a Copilot Studio agent.
///
/// Every method returns a stream of the agent's activities for that request; errors (settings,
/// token, transport, HTTP status, decoding) arrive as the stream's `Err` items. The trait is
/// object-safe. `conversation_id` is the one member beyond the upstream interfaces: the current
/// conversation is otherwise only observable through activities, and callers persisting it should
/// not have to depend on the concrete type.
pub trait CopilotClientApi: Send + Sync {
    /// Start a new conversation, optionally asking the agent to emit its start event.
    fn start_conversation(&self, emit_start_conversation_event: bool) -> ActivityStream<'_>;
    /// Start a new conversation with a full [`StartRequest`].
    fn start_conversation_with_request(&self, request: StartRequest) -> ActivityStream<'_>;
    /// Send a `message` with `question` in `conversation_id` (or the current conversation).
    fn ask_question(&self, question: &str, conversation_id: Option<&str>) -> ActivityStream<'_>;
    /// Send `activity` as the authenticated user.
    fn send_activity(&self, activity: Activity) -> ActivityStream<'_>;
    /// Send `activity` within `conversation_id`, forcing the activity's conversation to match.
    fn execute(&self, conversation_id: &str, activity: Activity) -> ActivityStream<'_>;
    /// Subscribe to a conversation's events, optionally resuming after `last_received_event_id`.
    fn subscribe(&self, conversation_id: &str, last_received_event_id: Option<&str>) -> SubscribeStream<'_>;
    /// The current conversation id, once one has been started or observed.
    fn conversation_id(&self) -> Option<String>;
}

impl CopilotClientApi for CopilotClient {
    fn start_conversation(&self, emit_start_conversation_event: bool) -> ActivityStream<'_> {
        CopilotClient::start_conversation(self, emit_start_conversation_event)
    }

    fn start_conversation_with_request(&self, request: StartRequest) -> ActivityStream<'_> {
        CopilotClient::start_conversation_with_request(self, request)
    }

    fn ask_question(&self, question: &str, conversation_id: Option<&str>) -> ActivityStream<'_> {
        CopilotClient::ask_question(self, question, conversation_id)
    }

    fn send_activity(&self, activity: Activity) -> ActivityStream<'_> {
        CopilotClient::send_activity(self, activity)
    }

    fn execute(&self, conversation_id: &str, activity: Activity) -> ActivityStream<'_> {
        CopilotClient::execute(self, conversation_id, activity)
    }

    fn subscribe(&self, conversation_id: &str, last_received_event_id: Option<&str>) -> SubscribeStream<'_> {
        CopilotClient::subscribe(self, conversation_id, last_received_event_id)
    }

    fn conversation_id(&self) -> Option<String> {
        CopilotClient::conversation_id(self)
    }
}

macro_rules! delegate_api {
    ($wrapper:ty) => {
        impl<T: CopilotClientApi + ?Sized> CopilotClientApi for $wrapper {
            fn start_conversation(&self, emit_start_conversation_event: bool) -> ActivityStream<'_> {
                (**self).start_conversation(emit_start_conversation_event)
            }
            fn start_conversation_with_request(&self, request: StartRequest) -> ActivityStream<'_> {
                (**self).start_conversation_with_request(request)
            }
            fn ask_question(&self, question: &str, conversation_id: Option<&str>) -> ActivityStream<'_> {
                (**self).ask_question(question, conversation_id)
            }
            fn send_activity(&self, activity: Activity) -> ActivityStream<'_> {
                (**self).send_activity(activity)
            }
            fn execute(&self, conversation_id: &str, activity: Activity) -> ActivityStream<'_> {
                (**self).execute(conversation_id, activity)
            }
            fn subscribe(&self, conversation_id: &str, last_received_event_id: Option<&str>) -> SubscribeStream<'_> {
                (**self).subscribe(conversation_id, last_received_event_id)
            }
            fn conversation_id(&self) -> Option<String> {
                (**self).conversation_id()
            }
        }
    };
}

delegate_api!(Arc<T>);
delegate_api!(Box<T>);
