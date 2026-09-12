//! Client behaviour against a mock HTTP server (docs/DESIGN.md §11.2).

use std::sync::{Arc, Mutex};

use copilotstudio_client::{
    Activity, ActivityType, BoxError, BoxFuture, ConnectionSettings, CopilotClient, Error, StartRequest, TokenProvider,
    headers,
};
use futures::TryStreamExt;
use serde_json::{Value, json};
use wiremock::matchers::{body_json, header, header_exists, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const BOT_PATH: &str = "/copilotstudio/dataverse-backed/authenticated/bots/Bot01";
const API_VERSION: &str = "2022-03-01-preview";

fn settings(server: &MockServer) -> ConnectionSettings {
    ConnectionSettings::direct(format!("{}{BOT_PATH}", server.uri()))
}

fn sse(activities: &[Value]) -> String {
    activities.iter().map(|a| format!("event: activity\ndata: {a}\n\n")).collect()
}

fn sse_response(activities: &[Value]) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(sse(activities), "text/event-stream")
}

fn message(text: &str, conversation_id: &str) -> Value {
    json!({"type": "message", "text": text, "conversation": {"id": conversation_id}})
}

async fn collect(stream: copilotstudio_client::ActivityStream<'_>) -> Vec<Activity> {
    stream.try_collect().await.unwrap()
}

#[tokio::test]
async fn start_conversation_streams_activities_and_captures_conversation_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations")))
        .and(query_param("api-version", API_VERSION))
        .and(header("accept", "text/event-stream"))
        .and(header("content-type", "application/json"))
        .and(header("authorization", "Bearer tok-1"))
        .and(body_json(json!({"emitStartConversationEvent": true})))
        .respond_with(
            sse_response(&[json!({"type": "typing", "conversation": {"id": "conv-1"}}), message("Welcome!", "conv-1")])
                .insert_header(headers::D2E_CONVERSATION_ID, "conv-1"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = CopilotClient::new(settings(&server), "tok-1");
    let activities = collect(client.start_conversation(true)).await;
    assert_eq!(activities.len(), 2);
    assert_eq!(activities[0].r#type, ActivityType::Typing);
    assert_eq!(activities[1].text.as_deref(), Some("Welcome!"));
    assert_eq!(client.conversation_id().as_deref(), Some("conv-1"));

    let requests = server.received_requests().await.unwrap();
    let ua = requests[0].headers.get("user-agent").unwrap().to_str().unwrap();
    assert!(ua.starts_with("CopilotStudioClient.agents-sdk-rust/"), "{ua}");
}

#[tokio::test]
async fn ask_question_posts_execute_turn_to_current_conversation() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations")))
        .respond_with(sse_response(&[]).insert_header(headers::D2E_CONVERSATION_ID, "conv-7"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations/conv-7")))
        .and(query_param("api-version", API_VERSION))
        .and(body_json(json!({"activity": {"type": "message", "text": "hello", "conversation": {"id": "conv-7"}}})))
        .respond_with(sse_response(&[message("hi back", "conv-7")]))
        .expect(1)
        .mount(&server)
        .await;

    let client = CopilotClient::new(settings(&server), "tok");
    collect(client.start_conversation(false)).await;
    let reply = collect(client.ask_question("hello", None)).await;
    assert_eq!(reply.len(), 1);
    assert_eq!(reply[0].text.as_deref(), Some("hi back"));
}

#[tokio::test]
async fn conversation_id_comes_from_first_message_when_header_absent_and_resets_on_start() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations")))
        .respond_with(sse_response(&[message("a", "from-activity"), message("b", "ignored-later")]))
        .mount(&server)
        .await;

    let client = CopilotClient::new(settings(&server), "tok");
    collect(client.start_conversation(true)).await;
    assert_eq!(client.conversation_id().as_deref(), Some("from-activity"));

    // A new start with a requested id resets the current conversation first (D1).
    let request = StartRequest { conversation_id: Some("requested".into()), ..Default::default() };
    collect(client.start_conversation_with_request(request)).await;
    assert_eq!(client.conversation_id().as_deref(), Some("requested"));
}

#[tokio::test]
async fn start_with_requested_conversation_id_sends_header_and_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations")))
        .and(header(headers::CONVERSATION_ID, "req-1"))
        .and(body_json(json!({"emitStartConversationEvent": false, "locale": "fr-FR", "conversationId": "req-1"})))
        .respond_with(sse_response(&[]))
        .expect(1)
        .mount(&server)
        .await;

    let client = CopilotClient::new(settings(&server), "tok");
    let request = StartRequest {
        emit_start_conversation_event: false,
        locale: Some("fr-FR".into()),
        conversation_id: Some("req-1".into()),
    };
    collect(client.start_conversation_with_request(request)).await;
}

#[tokio::test]
async fn execute_forces_conversation_id_and_makes_it_current() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations/forced")))
        .and(body_json(json!({"activity": {"type": "message", "text": "x", "conversation": {"id": "forced"}}})))
        .respond_with(sse_response(&[]))
        .expect(1)
        .mount(&server)
        .await;

    let client = CopilotClient::new(settings(&server), "tok");
    let activity = Activity::message("x").with_conversation_id("other");
    collect(client.execute("forced", activity)).await;
    assert_eq!(client.conversation_id().as_deref(), Some("forced"));

    let err = client.execute("", Activity::message("x")).try_next().await.unwrap_err();
    assert!(matches!(err, Error::InvalidArgument(_)), "{err}");
}

#[tokio::test]
async fn ask_question_without_a_known_conversation_sends_an_empty_conversation_account() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations")))
        .and(body_json(json!({"activity": {"type": "message", "text": "first", "conversation": {}}})))
        .respond_with(sse_response(&[]))
        .expect(1)
        .mount(&server)
        .await;
    let client = CopilotClient::new(settings(&server), "tok");
    collect(client.ask_question("first", None)).await;
}

#[tokio::test]
async fn send_activity_prefers_the_activity_conversation_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations/explicit")))
        .respond_with(sse_response(&[]))
        .expect(1)
        .mount(&server)
        .await;
    let client = CopilotClient::new(settings(&server), "tok");
    collect(client.send_activity(Activity::message("x").with_conversation_id("explicit"))).await;
}

#[tokio::test]
async fn island_header_is_ignored_when_a_direct_url_is_configured() {
    // Python `test_experimental_endpoint_not_captured_when_direct_connect_set`. The enabled /
    // disabled halves of the rule need a standard-mode host and live as unit tests in `client.rs`.
    let server = MockServer::start().await;
    let island = format!("{}/island/bot", server.uri());
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations")))
        .respond_with(
            sse_response(&[])
                .insert_header(headers::D2E_EXPERIMENTAL_URL, island.as_str())
                .insert_header(headers::D2E_CONVERSATION_ID, "c1"),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations/c1")))
        .respond_with(sse_response(&[message("still direct", "c1")]))
        .expect(1)
        .mount(&server)
        .await;

    let client = CopilotClient::new(settings(&server).use_experimental_endpoint(true), "tok");
    collect(client.start_conversation(true)).await;
    assert_eq!(client.island_experimental_url(), None);
    let reply = collect(client.ask_question("hi", None)).await;
    assert_eq!(reply[0].text.as_deref(), Some("still direct"));
}

#[tokio::test]
async fn json_fallback_bodies_are_parsed_for_start_and_execute() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"activities": [message("json start", "c9")], "conversationId": "c9"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations/c9")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"activities": [message("json turn", "c9"), message("second", "c9")]})),
        )
        .mount(&server)
        .await;

    let client = CopilotClient::new(settings(&server), "tok");
    let start = collect(client.start_conversation(true)).await;
    assert_eq!(start.len(), 1);
    assert_eq!(start[0].text.as_deref(), Some("json start"));
    let turn = collect(client.ask_question("q", Some("c9"))).await;
    assert_eq!(turn.iter().filter_map(|a| a.text.as_deref()).collect::<Vec<_>>(), vec!["json turn", "second"]);
}

#[tokio::test]
async fn event_stream_with_charset_parameter_is_still_sse() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations")))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(sse(&[message("ok", "c")]), "text/event-stream; charset=utf-8"),
        )
        .mount(&server)
        .await;
    let client = CopilotClient::new(settings(&server), "tok");
    assert_eq!(collect(client.start_conversation(true)).await.len(), 1);
}

#[tokio::test]
async fn non_activity_events_are_ignored_and_bad_json_is_a_decode_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations")))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            "event: ping\ndata: {}\n\nevent: activity\ndata: {\"type\":\"message\",\"text\":\"one\"}\n\nevent: end\ndata: done\n\nevent: activity\ndata: not json\n\n",
            "text/event-stream",
        ))
        .mount(&server)
        .await;
    let client = CopilotClient::new(settings(&server), "tok");
    let mut stream = client.start_conversation(true);
    let first = stream.try_next().await.unwrap().unwrap();
    assert_eq!(first.text.as_deref(), Some("one"));
    let err = stream.try_next().await.unwrap_err();
    assert!(matches!(err, Error::Decode { context: "activity", .. }), "{err}");
}

#[tokio::test]
async fn non_success_status_surfaces_status_and_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations")))
        .respond_with(ResponseTemplate::new(403).set_body_string("{\"error\":\"D2EAccessDenied\"}"))
        .mount(&server)
        .await;
    let client = CopilotClient::new(settings(&server), "tok");
    let err = client.start_conversation(true).try_next().await.unwrap_err();
    match err {
        Error::RequestFailed { status, body } => {
            assert_eq!(status.as_u16(), 403);
            assert_eq!(body, "{\"error\":\"D2EAccessDenied\"}");
        }
        other => panic!("unexpected {other}"),
    }
    assert_eq!(err_display(&server).await, "Error sending request: 403 Forbidden. {\"error\":\"D2EAccessDenied\"}");
}

async fn err_display(server: &MockServer) -> String {
    let client = CopilotClient::new(settings(server), "tok");
    client.start_conversation(true).try_next().await.unwrap_err().to_string()
}

struct RecordingProvider {
    urls: Arc<Mutex<Vec<String>>>,
    fail: bool,
}

impl TokenProvider for RecordingProvider {
    fn access_token(&self, request_url: &str) -> BoxFuture<'_, Result<String, BoxError>> {
        self.urls.lock().unwrap().push(request_url.to_owned());
        let fail = self.fail;
        Box::pin(async move { if fail { Err("no token for you".into()) } else { Ok("provided-token".to_owned()) } })
    }
}

#[tokio::test]
async fn token_provider_is_called_with_the_request_url_and_errors_surface() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations")))
        .and(header("authorization", "Bearer provided-token"))
        .respond_with(sse_response(&[]))
        .expect(1)
        .mount(&server)
        .await;

    let urls = Arc::new(Mutex::new(Vec::new()));
    let client = CopilotClient::builder(settings(&server))
        .token_provider(RecordingProvider { urls: urls.clone(), fail: false })
        .build()
        .unwrap();
    collect(client.start_conversation(true)).await;
    assert_eq!(
        urls.lock().unwrap().as_slice(),
        [format!("{}{BOT_PATH}/conversations?api-version={API_VERSION}", server.uri())]
    );

    let client = CopilotClient::builder(settings(&server))
        .token_provider(RecordingProvider { urls: urls.clone(), fail: true })
        .build()
        .unwrap();
    let err = client.start_conversation(true).try_next().await.unwrap_err();
    assert!(matches!(err, Error::Token(_)), "{err}");
    assert_eq!(err.to_string(), "token provider failed: no token for you");
}

#[tokio::test]
async fn builder_requires_a_token() {
    let err = CopilotClient::builder(ConnectionSettings::new("e", "s")).build().err().unwrap();
    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[tokio::test]
async fn subscribe_uses_get_with_last_event_id_and_surfaces_event_ids() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("{BOT_PATH}/conversations/c5/subscribe")))
        .and(query_param("api-version", API_VERSION))
        .and(header("last-event-id", "evt-9"))
        .and(header("accept", "text/event-stream"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            "id: evt-10\nevent: activity\ndata: {\"type\":\"message\",\"text\":\"a\"}\n\nevent: activity\ndata: {\"type\":\"typing\"}\n\n",
            "text/event-stream",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let client = CopilotClient::new(settings(&server), "tok");
    let events: Vec<_> = client.subscribe("c5", Some("evt-9")).try_collect().await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_id.as_deref(), Some("evt-10"));
    assert_eq!(events[0].activity.text.as_deref(), Some("a"));
    assert_eq!(events[1].event_id, None, "ids are per event; the second block carried none");

    let requests: Vec<Request> = server.received_requests().await.unwrap();
    assert_eq!(requests[0].headers.get("content-type").unwrap(), "application/json");
    assert!(requests[0].body.is_empty());

    let err = client.subscribe("  ", None).try_next().await.unwrap_err();
    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[tokio::test]
async fn subscribe_json_fallback() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("{BOT_PATH}/conversations/c6/subscribe")))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"activities": [message("late", "c6")]})))
        .mount(&server)
        .await;
    let client = CopilotClient::new(settings(&server), "tok");
    let events: Vec<_> = client.subscribe("c6", None).try_collect().await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, None);
}

#[tokio::test]
async fn client_is_shareable_across_tasks() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("{BOT_PATH}/conversations")))
        .respond_with(sse_response(&[message("hi", "c")]))
        .mount(&server)
        .await;
    let client = Arc::new(CopilotClient::new(settings(&server), "tok"));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let client = client.clone();
            tokio::spawn(async move { collect(client.start_conversation(true)).await.len() })
        })
        .collect();
    for h in handles {
        assert_eq!(h.await.unwrap(), 1);
    }
}
