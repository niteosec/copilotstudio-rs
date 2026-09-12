//! Rust half of the differential parity harness (`tests/differential/run.py`).
//!
//! Runs the same scenario as `tests/differential/scenario_python.py` against the recording server
//! and writes the yielded activities to `DIFFERENTIAL_OUT`. Ignored unless `DIFFERENTIAL_BASE_URL`
//! is set; `run.py` drives it and diffs the two recordings.

use copilotstudio_client::{Activity, ConnectionSettings, CopilotClient, StartRequest};
use futures::TryStreamExt;
use serde_json::{Map, Value, json};

fn client(base: &str, bot: &str) -> CopilotClient {
    CopilotClient::new(ConnectionSettings::direct(format!("{base}/bots/{bot}")), "tok-shared")
}

async fn collect(stream: copilotstudio_client::ActivityStream<'_>) -> Value {
    let activities: Vec<Activity> = stream.try_collect().await.expect("stream failed");
    serde_json::to_value(activities).unwrap()
}

#[tokio::test]
#[ignore = "driven by tests/differential/run.py"]
async fn scenario() {
    let Ok(base) = std::env::var("DIFFERENTIAL_BASE_URL") else { return };
    let out = std::env::var("DIFFERENTIAL_OUT").expect("DIFFERENTIAL_OUT");
    let mut steps = Map::new();

    let c = client(&base, "Main");
    steps.insert("start".into(), collect(c.start_conversation(true)).await);
    steps.insert("conversation_id_after_start".into(), json!(c.conversation_id().unwrap_or_default()));
    steps.insert("ask".into(), collect(c.ask_question("Hello?", None)).await);
    steps.insert("execute".into(), collect(c.execute("explicit-B", Activity::message("run this"))).await);
    steps.insert("conversation_id_after_execute".into(), json!(c.conversation_id().unwrap_or_default()));
    let subscribe: Vec<_> = c.subscribe("conv-A", Some("evt-0")).try_collect().await.expect("subscribe failed");
    steps.insert(
        "subscribe".into(),
        json!(subscribe.iter().map(|e| json!({"event_id": e.event_id, "activity": e.activity})).collect::<Vec<_>>()),
    );
    let request = StartRequest {
        emit_start_conversation_event: false,
        locale: Some("fr-FR".into()),
        conversation_id: Some("conv-A".into()),
    };
    steps.insert("start_with_request".into(), collect(c.start_conversation_with_request(request)).await);

    let c = client(&base, "NoHeader");
    steps.insert("headerless_start".into(), collect(c.start_conversation(true)).await);
    steps.insert("conversation_id_after_headerless_start".into(), json!(c.conversation_id().unwrap_or_default()));
    steps.insert("headerless_ask".into(), collect(c.ask_question("which?", None)).await);

    let c = client(&base, "JsonOnly");
    steps.insert("json_only_start".into(), collect(c.start_conversation(true)).await);
    steps.insert("conversation_id_after_json_only".into(), json!(c.conversation_id().unwrap_or_default()));

    let c = client(&base, "Broken");
    let broken = match c.start_conversation(true).try_collect::<Vec<_>>().await {
        Ok(_) => "no error".to_owned(),
        Err(e) => format!("{}: {e}", std::any::type_name_of_val(&e).rsplit("::").next().unwrap_or("Error")),
    };
    steps.insert("broken_start".into(), json!(broken));

    std::fs::write(&out, serde_json::to_string_pretty(&Value::Object(steps)).unwrap()).expect("write out");
}
