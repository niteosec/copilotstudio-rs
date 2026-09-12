//! `CopilotClientApi` is object-safe, is implemented by the real client, and can be replaced by a
//! test double — the reason the trait exists.

use std::sync::{Arc, Mutex};

use copilotstudio_client::{
    Activity, ActivityStream, ActivityType, ConnectionSettings, CopilotClient, CopilotClientApi, Error, StartRequest,
    SubscribeStream,
};
use futures::{TryStreamExt, stream};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A scripted double: records what it was asked and answers with canned activities.
#[derive(Default)]
struct Scripted {
    asked: Mutex<Vec<String>>,
}

impl CopilotClientApi for Scripted {
    fn start_conversation(&self, _emit: bool) -> ActivityStream<'_> {
        Box::pin(stream::iter([Ok(Activity::message("welcome").with_conversation_id("scripted"))]))
    }
    fn start_conversation_with_request(&self, request: StartRequest) -> ActivityStream<'_> {
        self.start_conversation(request.emit_start_conversation_event)
    }
    fn ask_question(&self, question: &str, _conversation_id: Option<&str>) -> ActivityStream<'_> {
        self.asked.lock().unwrap().push(question.to_owned());
        Box::pin(stream::iter([Ok(Activity::message(format!("echo: {question}")).with_conversation_id("scripted"))]))
    }
    fn send_activity(&self, activity: Activity) -> ActivityStream<'_> {
        Box::pin(stream::iter([Ok(activity)]))
    }
    fn execute(&self, conversation_id: &str, activity: Activity) -> ActivityStream<'_> {
        self.send_activity(activity.with_conversation_id(conversation_id))
    }
    fn subscribe(&self, _conversation_id: &str, _last: Option<&str>) -> SubscribeStream<'_> {
        Box::pin(stream::iter([Err(Error::InvalidArgument("scripted double does not subscribe"))]))
    }
    fn conversation_id(&self) -> Option<String> {
        Some("scripted".into())
    }
}

/// Consumer code written against the trait only.
async fn greet_and_ask(client: &dyn CopilotClientApi, question: &str) -> Result<Vec<String>, Error> {
    let mut texts = Vec::new();
    let mut s = client.start_conversation(true);
    while let Some(a) = s.try_next().await? {
        texts.extend(a.text);
    }
    let mut s = client.ask_question(question, None);
    while let Some(a) = s.try_next().await? {
        if a.r#type == ActivityType::Message {
            texts.extend(a.text);
        }
    }
    Ok(texts)
}

#[tokio::test]
async fn consumer_runs_against_the_double() {
    let double = Arc::new(Scripted::default());
    let shared: Arc<dyn CopilotClientApi> = double.clone();
    let texts = greet_and_ask(shared.as_ref(), "ping").await.unwrap();
    assert_eq!(texts, vec!["welcome", "echo: ping"]);
    assert_eq!(double.asked.lock().unwrap().as_slice(), ["ping"]);
    assert_eq!(shared.conversation_id().as_deref(), Some("scripted"));
}

#[tokio::test]
async fn consumer_runs_against_the_real_client_through_the_trait() {
    let server = MockServer::start().await;
    let bot = "/copilotstudio/dataverse-backed/authenticated/bots/Bot01";
    let sse = |text: &str| {
        format!(
            "event: activity\ndata: {{\"type\":\"message\",\"text\":\"{text}\",\"conversation\":{{\"id\":\"c1\"}}}}\n\n"
        )
    };
    Mock::given(method("POST"))
        .and(path(format!("{bot}/conversations")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(sse("hello"), "text/event-stream")
                .insert_header("x-ms-conversationid", "c1"),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{bot}/conversations/c1")))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse("pong"), "text/event-stream"))
        .mount(&server)
        .await;

    let client: Box<dyn CopilotClientApi> =
        Box::new(CopilotClient::new(ConnectionSettings::direct(format!("{}{bot}", server.uri())), "tok"));
    let texts = greet_and_ask(client.as_ref(), "ping").await.unwrap();
    assert_eq!(texts, vec!["hello", "pong"]);
    assert_eq!(client.conversation_id().as_deref(), Some("c1"));
}

#[tokio::test]
async fn errors_flow_through_the_trait_as_stream_items() {
    let double = Scripted::default();
    let err = double.subscribe("c", None).try_next().await.unwrap_err();
    assert!(matches!(err, Error::InvalidArgument(_)));
}
