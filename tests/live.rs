//! Live smoke test against a real Copilot Studio agent (docs/DESIGN.md §11.3).
//!
//! Ignored unless all of these are set:
//!   COPILOTSTUDIO_TOKEN            a delegated (or OBO / app-only) access token for the audience
//!                                  `scope_from_settings` returns (default `https://api.powerplatform.com/.default`)
//!   ENVIRONMENT_ID + SCHEMA_NAME   or DIRECT_CONNECT_URL   (see `ConnectionSettings::from_env`)
//!
//! Run with: `cargo test --test live -- --ignored --nocapture`

use copilotstudio_client::{ActivityType, ConnectionSettings, CopilotClient};
use futures::TryStreamExt;

fn live_settings() -> Option<(ConnectionSettings, String)> {
    let token = std::env::var("COPILOTSTUDIO_TOKEN").ok().filter(|t| !t.is_empty())?;
    let settings = ConnectionSettings::from_env().ok()?;
    let addressable =
        settings.direct_connect_url.is_some() || (settings.environment_id.is_some() && settings.schema_name.is_some());
    addressable.then_some((settings, token))
}

#[tokio::test]
#[ignore = "needs COPILOTSTUDIO_TOKEN and agent settings in the environment"]
async fn start_conversation_then_send_message_round_trip() {
    let Some((settings, token)) = live_settings() else {
        eprintln!("live test skipped: COPILOTSTUDIO_TOKEN / ENVIRONMENT_ID / SCHEMA_NAME not set");
        return;
    };
    let client = CopilotClient::new(settings.clone().enable_diagnostics(true), token);
    eprintln!("scope: {}", copilotstudio_client::scope_from_settings(&settings).unwrap());

    let start: Vec<_> = client.start_conversation(true).try_collect().await.expect("start_conversation");
    eprintln!("start yielded {} activities; conversation {:?}", start.len(), client.conversation_id());
    assert!(client.conversation_id().is_some(), "no conversation id after start");

    let reply: Vec<_> = client
        .ask_question("Hello! In one sentence, what can you help me with?", None)
        .try_collect()
        .await
        .expect("ask_question");
    for a in &reply {
        eprintln!("{:<20} {}", a.r#type, a.text.as_deref().unwrap_or(""));
    }
    let message =
        reply.iter().find(|a| a.r#type == ActivityType::Message && a.text.as_deref().is_some_and(|t| !t.is_empty()));
    assert!(message.is_some(), "no message activity with text in the reply: {reply:?}");
}
