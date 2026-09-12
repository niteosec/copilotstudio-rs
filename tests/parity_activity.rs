//! Activity-schema parity: fixtures in `tests/fixtures/` are produced by the upstream Python model
//! (`tests/fixtures/generate.py`) with `model_dump_json(exclude_unset=True, by_alias=True)` — the
//! bytes the reference client puts on the wire. Each must deserialise into the Rust types and
//! re-serialise to the identical JSON value.

use copilotstudio_client::activity::{ChannelId, Entity, constants};
use copilotstudio_client::{Activity, ActivityType, ExecuteTurnRequest, StartRequest};
use serde_json::{Value, json};

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))).unwrap()
}

fn assert_round_trip<T: serde::de::DeserializeOwned + serde::Serialize>(name: &str) -> T {
    let expected = fixture(name);
    let parsed: T = serde_json::from_value(expected.clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
    let actual = serde_json::to_value(&parsed).unwrap();
    assert_eq!(actual, expected, "{name} did not round-trip");
    parsed
}

#[test]
fn minimal_message_round_trips() {
    let a: Activity = assert_round_trip("activity_minimal_message.json");
    assert_eq!(a.r#type, ActivityType::Message);
    assert_eq!(a.text.as_deref(), Some("Hello, world!"));
    assert_eq!(a.conversation_id(), Some("1234567890"));
}

#[test]
fn rich_activity_round_trips_with_aliases_entities_and_opaque_json() {
    let a: Activity = assert_round_trip("activity_rich.json");
    // `from` ↔ from_property, `bot` ↔ relates_to.agent
    assert_eq!(a.from_property.as_ref().and_then(|f| f.id.as_deref()), Some("agent-1"));
    assert_eq!(a.relates_to.as_ref().and_then(|r| r.agent.as_ref()).and_then(|b| b.id.as_deref()), Some("agent-1"));
    // channelId is a plain string; the upstream model moved the sub-channel into a ProductInfo entity
    assert_eq!(a.channel_id, Some(ChannelId::from("msteams")));
    let entities = a.entities.as_deref().unwrap();
    assert_eq!(entities.len(), 4);
    assert_eq!(entities[1].get("@type"), Some(&json!("Message")));
    assert_eq!(entities[2].get("nested"), Some(&json!({"a": [1, 2]})));
    assert_eq!(entities[3].r#type, constants::entity_types::PRODUCT_INFO);
    // timestamps are kept verbatim (D5)
    assert_eq!(a.timestamp.as_deref(), Some("2026-09-12T10:11:12.123456Z"));
    // the final-message stream info is readable
    let info = a.stream_info().unwrap();
    assert_eq!(info.stream_type.as_deref(), Some("final"));
    assert_eq!(info.stream_sequence, Some(3));
    // opaque JSON survives
    assert_eq!(a.value, Some(json!({"k": "v", "n": 1})));
    assert_eq!(a.attachments.as_deref().unwrap()[0].content.as_ref().unwrap()["type"], "AdaptiveCard");
    assert_eq!(a.suggested_actions.as_ref().unwrap().actions.as_deref().unwrap()[0].value, Some(json!("yes")));
}

#[test]
fn typing_stream_chunk_exposes_stream_info() {
    let a: Activity = assert_round_trip("activity_typing_stream_chunk.json");
    assert_eq!(a.r#type, ActivityType::Typing);
    let info = a.stream_info().unwrap();
    assert_eq!(info.stream_id.as_deref(), Some("stream-1"));
    assert_eq!(info.stream_sequence, Some(2));
    assert_eq!(info.stream_type.as_deref(), Some("streaming"));
}

#[test]
fn legacy_channel_data_stream_info_is_read_when_no_entity() {
    let a: Activity = serde_json::from_value(json!({
        "type": "typing",
        "text": "Hel",
        "channelData": {"streamType": "streaming", "streamId": "s", "streamSequence": 1}
    }))
    .unwrap();
    let info = a.stream_info().unwrap();
    assert_eq!(info.stream_id.as_deref(), Some("s"));
    assert_eq!(info.stream_sequence, Some(1));
}

#[test]
fn unknown_top_level_properties_are_preserved() {
    let raw = json!({"type": "event", "name": "x", "zzzUnknown": {"a": 1}, "another": true});
    let a: Activity = serde_json::from_value(raw.clone()).unwrap();
    assert_eq!(a.properties.get("zzzUnknown"), Some(&json!({"a": 1})));
    assert_eq!(serde_json::to_value(&a).unwrap(), raw);
}

#[test]
fn unknown_activity_type_is_preserved() {
    let a: Activity = serde_json::from_value(json!({"type": "somethingNew"})).unwrap();
    assert_eq!(a.r#type, ActivityType::Other("somethingNew".into()));
    assert_eq!(serde_json::to_value(&a).unwrap(), json!({"type": "somethingNew"}));
}

#[test]
fn message_constructor_serialises_only_what_is_set() {
    assert_eq!(serde_json::to_value(Activity::message("hi")).unwrap(), json!({"type": "message", "text": "hi"}));
    let a = Activity::message("hi").with_conversation_id("c1");
    assert_eq!(
        serde_json::to_value(&a).unwrap(),
        json!({"type": "message", "text": "hi", "conversation": {"id": "c1"}})
    );
}

#[test]
fn execute_turn_request_matches_upstream_body() {
    let req = ExecuteTurnRequest { activity: Activity::message("Test message").with_conversation_id("456") };
    assert_eq!(serde_json::to_value(&req).unwrap(), fixture("execute_turn_request.json"));
    assert_round_trip::<ExecuteTurnRequest>("execute_turn_request.json");
}

#[test]
fn start_request_matches_upstream_bodies() {
    let full = StartRequest {
        emit_start_conversation_event: true,
        locale: Some("en-US".into()),
        conversation_id: Some("test-123".into()),
    };
    assert_eq!(serde_json::to_value(&full).unwrap(), fixture("start_request_full.json"));
    assert_eq!(serde_json::to_value(StartRequest::default()).unwrap(), fixture("start_request_default.json"));
    assert_eq!(serde_json::to_value(StartRequest::new(false)).unwrap(), json!({"emitStartConversationEvent": false}));
}

#[test]
fn channel_id_splits_channel_and_sub_channel() {
    let id = ChannelId::from("msteams:sub");
    assert_eq!(id.channel(), "msteams");
    assert_eq!(id.sub_channel(), Some("sub"));
    assert_eq!(ChannelId::from("directline").sub_channel(), None);
    assert_eq!(ChannelId::new("email", Some("work")).to_string(), "email:work");
    assert_eq!(serde_json::to_value(ChannelId::from("a:b")).unwrap(), json!("a:b"));
}

#[test]
fn entity_view_as_stream_info_ignores_type() {
    let e: Entity =
        serde_json::from_value(json!({"type": "streaminfo", "streamId": "x", "streamSequence": 7})).unwrap();
    assert_eq!(e.as_stream_info().unwrap().stream_sequence, Some(7));
}
