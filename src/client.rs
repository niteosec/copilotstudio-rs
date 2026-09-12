//! `CopilotClient` — the Direct-to-Engine client.
//!
//! Mirrors `CopilotClient.cs` (.NET, primary), `copilot_client.py` and `copilotStudioClient.ts`.
//! Every operation is one HTTP request whose response is surfaced as a stream of activities;
//! dropping the stream cancels the request.

use std::fmt;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures_core::Stream;
use futures_util::StreamExt;
use http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use http::{HeaderMap, HeaderValue, Method};
use url::Url;

use crate::activity::{Activity, ActivityType, ConversationAccount};
use crate::connection_settings::ConnectionSettings;
use crate::error::{Error, SettingsError};
use crate::headers;
use crate::models::{
    ExecuteTurnRequest, ExecuteTurnResponse, StartRequest, StartResponse, SubscribeEvent, SubscribeResponse,
};
use crate::power_platform_environment::{connection_url, scope_from_settings, subscribe_url};
use crate::sse::{SseEvent, SseParser};
use crate::token::{StaticToken, TokenProvider};
use crate::user_agent::user_agent;

/// A stream of activities from the agent.
pub type ActivityStream<'a> = Pin<Box<dyn Stream<Item = Result<Activity, Error>> + Send + 'a>>;
/// A stream of subscription events.
pub type SubscribeStream<'a> = Pin<Box<dyn Stream<Item = Result<SubscribeEvent, Error>> + Send + 'a>>;

/// Which request is in flight — selects the JSON-fallback body shape (`RequestTypes.cs`).
/// `ContinueSession` (subscribe) decodes on its own path because it also carries event ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestType {
    StartSession,
    ExecuteAction,
}

#[derive(Debug, Default)]
struct State {
    conversation_id: String,
    island_experimental_url: Option<String>,
}

/// Client for a Copilot Studio agent over the Direct-to-Engine API.
///
/// Construct with [`CopilotClient::new`] (fixed token) or [`CopilotClient::builder`]
/// (token provider, custom `reqwest::Client`). Share across tasks through `Arc`.
pub struct CopilotClient {
    settings: ConnectionSettings,
    http: reqwest::Client,
    token: Arc<dyn TokenProvider>,
    state: Mutex<State>,
}

impl fmt::Debug for CopilotClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CopilotClient")
            .field("settings", &self.settings)
            .field("conversation_id", &self.conversation_id())
            .field("token", &"[REDACTED]")
            .finish()
    }
}

/// Builder for [`CopilotClient`].
pub struct CopilotClientBuilder {
    settings: ConnectionSettings,
    http: Option<reqwest::Client>,
    token: Option<Arc<dyn TokenProvider>>,
}

impl CopilotClientBuilder {
    /// Use a fixed bearer token.
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(Arc::new(StaticToken::new(token)));
        self
    }

    /// Use a [`TokenProvider`], called once per request with the request URL.
    pub fn token_provider(mut self, provider: impl TokenProvider + 'static) -> Self {
        self.token = Some(Arc::new(provider));
        self
    }

    /// Use a caller-configured `reqwest::Client` (proxies, timeouts, TLS roots, …).
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http = Some(client);
        self
    }

    /// Build the client. Fails only when no token or token provider was supplied.
    pub fn build(self) -> Result<CopilotClient, Error> {
        let token = self.token.ok_or(Error::InvalidArgument("a token or token provider must be supplied"))?;
        Ok(CopilotClient {
            settings: self.settings,
            http: self.http.unwrap_or_default(),
            token,
            state: Mutex::new(State::default()),
        })
    }
}

impl CopilotClient {
    /// A client that sends `token` as the bearer token on every request (the Python / JS shape).
    pub fn new(settings: ConnectionSettings, token: impl Into<String>) -> Self {
        Self {
            settings,
            http: reqwest::Client::new(),
            token: Arc::new(StaticToken::new(token)),
            state: Mutex::new(State::default()),
        }
    }

    /// Start building a client.
    pub fn builder(settings: ConnectionSettings) -> CopilotClientBuilder {
        CopilotClientBuilder { settings, http: None, token: None }
    }

    /// The settings this client was built with (the captured experimental URL is not applied here;
    /// see [`island_experimental_url`](Self::island_experimental_url)).
    pub fn settings(&self) -> &ConnectionSettings {
        &self.settings
    }

    /// The current conversation id, once one has been started or observed.
    pub fn conversation_id(&self) -> Option<String> {
        let id = &self.lock().conversation_id;
        (!id.is_empty()).then(|| id.clone())
    }

    /// The island experimental URL captured from `x-ms-d2e-experimental`, if any.
    pub fn island_experimental_url(&self) -> Option<String> {
        self.lock().island_experimental_url.clone()
    }

    /// The token audience the caller should request for these settings
    /// (`scope_from_settings(self.settings())`).
    pub fn scope(&self) -> Result<String, SettingsError> {
        scope_from_settings(&self.settings)
    }

    // ---- Start conversation ---------------------------------------------------------------

    /// Start a new conversation, optionally asking the agent to emit its start event.
    pub fn start_conversation(&self, emit_start_conversation_event: bool) -> ActivityStream<'_> {
        self.start_conversation_with_request(StartRequest::new(emit_start_conversation_event))
    }

    /// Start a new conversation with a full [`StartRequest`] (locale, requested conversation id).
    pub fn start_conversation_with_request(&self, request: StartRequest) -> ActivityStream<'_> {
        Box::pin(async_stream::try_stream! {
            // A start request establishes a new current conversation (docs/DESIGN.md D1).
            self.lock().conversation_id = request.conversation_id.clone().unwrap_or_default();
            let url = connection_url(&self.effective_settings(), None)?;
            let mut extra = HeaderMap::new();
            if let Some(id) = request.conversation_id.as_deref().filter(|s| !s.is_empty()) {
                extra.insert(headers::CONVERSATION_ID, header_value(id)?);
            }
            let body = serde_json::to_vec(&request).map_err(|e| Error::decode("StartRequest", e))?;
            let response = self.send(Method::POST, url, Some(body), extra).await?;
            let mut activities = self.activities(response, RequestType::StartSession);
            while let Some(activity) = activities.next().await {
                yield activity?;
            }
        })
    }

    // ---- Send activity ----------------------------------------------------------------------

    /// Send a `message` with `question` in `conversation_id` (or the current conversation).
    pub fn ask_question(&self, question: impl Into<String>, conversation_id: Option<&str>) -> ActivityStream<'_> {
        let id = conversation_id
            .map(str::to_owned)
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| self.lock().conversation_id.clone());
        let activity = Activity { conversation: Some(ConversationAccount::new(id)), ..Activity::message(question) };
        self.send_activity(activity)
    }

    /// Send `activity` as the authenticated user. Uses `activity.conversation.id` when set, else
    /// the current conversation id.
    pub fn send_activity(&self, activity: Activity) -> ActivityStream<'_> {
        Box::pin(async_stream::try_stream! {
            let local_id = match activity.conversation_id() {
                Some(id) => id.to_owned(),
                None => self.lock().conversation_id.clone(),
            };
            let url = connection_url(&self.effective_settings(), Some(&local_id).filter(|s| !s.is_empty()).map(String::as_str))?;
            let body = serde_json::to_vec(&ExecuteTurnRequest { activity }).map_err(|e| Error::decode("ExecuteTurnRequest", e))?;
            let response = self.send(Method::POST, url, Some(body), HeaderMap::new()).await?;
            let mut activities = self.activities(response, RequestType::ExecuteAction);
            while let Some(activity) = activities.next().await {
                yield activity?;
            }
        })
    }

    /// Send `activity` within `conversation_id`, forcing `activity.conversation.id` to match and
    /// making it the current conversation.
    pub fn execute(&self, conversation_id: &str, mut activity: Activity) -> ActivityStream<'_> {
        if conversation_id.trim().is_empty() {
            return Box::pin(futures_util::stream::once(async {
                Err(Error::InvalidArgument("CopilotClient.execute: conversation_id cannot be empty"))
            }));
        }
        activity.set_conversation_id(conversation_id);
        self.lock().conversation_id = conversation_id.to_owned();
        self.send_activity(activity)
    }

    // ---- Subscribe --------------------------------------------------------------------------

    /// Subscribe to a conversation's events over SSE, optionally resuming after
    /// `last_received_event_id`.
    ///
    /// Upstream marks this API as available to Microsoft-internal callers only at this time.
    pub fn subscribe(&self, conversation_id: &str, last_received_event_id: Option<&str>) -> SubscribeStream<'_> {
        let conversation_id = conversation_id.to_owned();
        let last_event_id = last_received_event_id.map(str::to_owned).filter(|s| !s.is_empty());
        Box::pin(async_stream::try_stream! {
            if conversation_id.trim().is_empty() {
                Err(Error::InvalidArgument("CopilotClient.subscribe: conversation_id cannot be empty"))?;
            }
            let url = subscribe_url(&self.effective_settings(), &conversation_id)?;
            let mut extra = HeaderMap::new();
            if let Some(id) = &last_event_id {
                extra.insert(headers::LAST_EVENT_ID, header_value(id)?);
            }
            let response = self.send(Method::GET, url, None, extra).await?;
            if is_event_stream(&response) {
                let mut body = response.bytes_stream();
                let mut parser = SseParser::new();
                while let Some(chunk) = body.next().await {
                    for event in parser.feed(&chunk?) {
                        if let Some(activity) = decode_activity_event(&event)? {
                            yield SubscribeEvent { activity, event_id: event.id };
                        }
                    }
                }
                if let Some(event) = parser.finish() {
                    if let Some(activity) = decode_activity_event(&event)? {
                        yield SubscribeEvent { activity, event_id: event.id };
                    }
                }
            } else {
                tracing::warn!("Expected an event stream but did not receive one. Attempting to parse response content as JSON.");
                let bytes = response.bytes().await?;
                let parsed: SubscribeResponse = serde_json::from_slice(&bytes).map_err(|e| Error::decode("SubscribeResponse", e))?;
                for activity in parsed.activities {
                    yield SubscribeEvent { activity, event_id: None };
                }
            }
        })
    }

    // ---- Internals --------------------------------------------------------------------------

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Settings with the captured island URL applied (upstream mutates `settings.direct_connect_url`).
    fn effective_settings(&self) -> ConnectionSettings {
        let mut settings = self.settings.clone();
        if let Some(url) = &self.lock().island_experimental_url {
            settings.direct_connect_url = Some(url.clone());
        }
        settings
    }

    /// `SetupAndExecutePostRequest`: authenticate, send, fail on non-2xx, capture response headers.
    async fn send(
        &self,
        method: Method,
        url: Url,
        body: Option<Vec<u8>>,
        extra: HeaderMap,
    ) -> Result<reqwest::Response, Error> {
        let token = self.token.access_token(url.as_str()).await.map_err(Error::Token)?;
        let mut request = self
            .http
            .request(method, url.clone())
            .header(ACCEPT, headers::EVENT_STREAM)
            .header(USER_AGENT, user_agent())
            .headers(extra);
        if !token.is_empty() {
            request = request.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some(body) = body {
            request = request.header(CONTENT_TYPE, headers::APPLICATION_JSON).body(body);
        }
        if self.settings.enable_diagnostics {
            tracing::debug!(url = %url, ">>> SEND TO {url}");
        }

        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::error!(%status, "Error sending request: {status}. {body}");
            return Err(Error::RequestFailed { status, body });
        }

        self.process_response_headers(response.headers());
        Ok(response)
    }

    /// Capture `x-ms-d2e-experimental` (only when opted in and no direct URL is configured) and
    /// `x-ms-conversationid`; log the headers when diagnostics are on (`Authorization` excluded).
    fn process_response_headers(&self, response_headers: &HeaderMap) {
        if let Some(experimental) = header_str(response_headers, headers::D2E_EXPERIMENTAL_URL) {
            if self.settings.use_experimental_endpoint
                && self.settings.direct_connect_url.as_deref().is_none_or(str::is_empty)
            {
                tracing::trace!("Island Experimental URL: {experimental}");
                self.lock().island_experimental_url = Some(experimental.to_owned());
            }
        }
        if let Some(id) = header_str(response_headers, headers::D2E_CONVERSATION_ID) {
            tracing::trace!("Conversation ID: {id}");
            self.lock().conversation_id = id.to_owned();
        }
        if self.settings.enable_diagnostics {
            tracing::debug!("=====================================================");
            for (name, value) in response_headers {
                if name == AUTHORIZATION {
                    continue;
                }
                tracing::debug!("{name} = {}", value.to_str().unwrap_or("<binary>"));
            }
            tracing::debug!("=====================================================");
        }
    }

    /// `PostActivityRequestAsync`: SSE when streamed, JSON fallback otherwise.
    fn activities(&self, response: reqwest::Response, request_type: RequestType) -> ActivityStream<'_> {
        Box::pin(async_stream::try_stream! {
            if is_event_stream(&response) {
                let mut body = response.bytes_stream();
                let mut parser = SseParser::new();
                while let Some(chunk) = body.next().await {
                    for event in parser.feed(&chunk?) {
                        if let Some(activity) = decode_activity_event(&event)? {
                            self.observe(&activity);
                            yield activity;
                        }
                    }
                }
                if let Some(event) = parser.finish() {
                    if let Some(activity) = decode_activity_event(&event)? {
                        self.observe(&activity);
                        yield activity;
                    }
                }
            } else {
                tracing::warn!("Expected an event stream but did not receive one. Attempting to parse response content as JSON.");
                let bytes = response.bytes().await?;
                let activities = match request_type {
                    RequestType::StartSession => {
                        serde_json::from_slice::<StartResponse>(&bytes).map_err(|e| Error::decode("StartResponse", e))?.activities
                    }
                    RequestType::ExecuteAction => {
                        serde_json::from_slice::<ExecuteTurnResponse>(&bytes).map_err(|e| Error::decode("ExecuteTurnResponse", e))?.activities
                    }
                };
                for activity in activities {
                    yield activity;
                }
            }
        })
    }

    /// Only set the conversation id from the first `message` when the header did not supply it.
    fn observe(&self, activity: &Activity) {
        if activity.r#type == ActivityType::Message {
            if let Some(id) = activity.conversation_id() {
                let mut state = self.lock();
                if state.conversation_id.is_empty() {
                    tracing::info!("Conversation ID: {id}");
                    state.conversation_id = id.to_owned();
                }
            }
        }
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok()).map(str::trim).filter(|s| !s.is_empty())
}

fn header_value(value: &str) -> Result<HeaderValue, Error> {
    HeaderValue::from_str(value).map_err(|_| Error::InvalidArgument("header value contains invalid characters"))
}

fn is_event_stream(response: &reqwest::Response) -> bool {
    header_str(response.headers(), CONTENT_TYPE.as_str()).is_some_and(|ct| {
        ct.split(';').next().is_some_and(|mime| mime.trim().eq_ignore_ascii_case(headers::EVENT_STREAM))
    })
}

/// Decode the activity carried by an `activity` SSE event; other event types yield `None`.
fn decode_activity_event(event: &SseEvent) -> Result<Option<Activity>, Error> {
    if event.event.as_deref() != Some("activity") {
        return Ok(None);
    }
    serde_json::from_str(&event.data).map(Some).map_err(|e| Error::decode("activity", e))
}

#[cfg(test)]
mod tests {
    //! The island-URL capture rule (Python `test_experimental_endpoint_*`). Standard-mode hosts are
    //! not reachable from a mock server, so the rule is exercised on the header processor directly.

    use super::*;

    const ISLAND: &str = "https://experimental.api.powerplatform.com/bot/test-bot";

    fn headers_with_island() -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(headers::D2E_CONVERSATION_ID, HeaderValue::from_static("test-conv-123"));
        h.insert(headers::D2E_EXPERIMENTAL_URL, HeaderValue::from_static(ISLAND));
        h
    }

    #[test]
    fn captured_when_enabled_and_no_direct_url_and_switches_effective_settings() {
        let settings = ConnectionSettings::new("environment-id", "agent-id").use_experimental_endpoint(true);
        let client = CopilotClient::new(settings, "token");
        client.process_response_headers(&headers_with_island());
        assert_eq!(client.island_experimental_url().as_deref(), Some(ISLAND));
        assert_eq!(client.conversation_id().as_deref(), Some("test-conv-123"));
        assert_eq!(client.effective_settings().direct_connect_url.as_deref(), Some(ISLAND));
        assert_eq!(client.settings().direct_connect_url, None, "the configured settings are not mutated");
        let next = connection_url(&client.effective_settings(), Some("test-conv-123")).unwrap();
        assert_eq!(
            next.as_str(),
            "https://experimental.api.powerplatform.com/bot/test-bot/conversations/test-conv-123?api-version=2022-03-01-preview"
        );
    }

    #[test]
    fn not_captured_when_disabled() {
        let client = CopilotClient::new(ConnectionSettings::new("environment-id", "agent-id"), "token");
        client.process_response_headers(&headers_with_island());
        assert_eq!(client.island_experimental_url(), None);
        assert_eq!(client.effective_settings().direct_connect_url, None);
    }

    #[test]
    fn not_captured_when_direct_connect_url_is_set() {
        let direct = "https://direct.api.powerplatform.com/bot/direct-bot";
        let settings = ConnectionSettings::new("environment-id", "agent-id")
            .direct_connect_url(direct)
            .use_experimental_endpoint(true);
        let client = CopilotClient::new(settings, "token");
        client.process_response_headers(&headers_with_island());
        assert_eq!(client.island_experimental_url(), None);
        assert_eq!(client.effective_settings().direct_connect_url.as_deref(), Some(direct));
    }

    #[test]
    fn debug_never_prints_the_token() {
        let client = CopilotClient::new(ConnectionSettings::new("e", "s"), "super-secret");
        let debug = format!("{client:?}");
        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("[REDACTED]"));
    }
}
