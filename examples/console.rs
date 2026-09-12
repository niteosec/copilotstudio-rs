//! Console chat with a Copilot Studio agent — the Rust twin of the upstream console samples
//! (`CopilotStudioClientSample`, `copilot_studio_client_sample`, `copilotstudio-console`).
//!
//! Token acquisition is outside the library, exactly as upstream: this example either takes a
//! pre-acquired token (`COPILOTSTUDIO_TOKEN`) or runs the Microsoft identity platform **device
//! code** flow for a public-client app registration that has the delegated
//! `CopilotStudio.Copilots.Invoke` permission on the Power Platform API.
//!
//! ```text
//! ENVIRONMENT_ID=… SCHEMA_NAME=… TENANT_ID=… APP_CLIENT_ID=… cargo run --example console
//! # or
//! ENVIRONMENT_ID=… SCHEMA_NAME=… COPILOTSTUDIO_TOKEN=… cargo run --example console
//! ```
//! Optional: `CLOUD`, `COPILOT_AGENT_TYPE`, `DIRECT_CONNECT_URL`, `LOCALE`, `ENABLE_DIAGNOSTICS=true`.

mod common;

use std::io::Write;

use copilotstudio_client::activity::constants::text_format_types;
use copilotstudio_client::{ActivityType, ConnectionSettings, CopilotClient, StartRequest};
use futures::TryStreamExt;
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = ConnectionSettings::from_env()?;
    if settings.enable_diagnostics {
        // Upstream's EnableDiagnostics writes to the console log sink; here that is `tracing`.
        tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).with_writer(std::io::stderr).init();
    }
    let scope = copilotstudio_client::scope_from_settings(&settings)?;

    let token = match std::env::var("COPILOTSTUDIO_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => common::device_code_login(&scope).await?,
    };
    let client = CopilotClient::new(settings, token);

    let locale = std::env::var("LOCALE").ok().filter(|l| !l.is_empty());
    let start = StartRequest { locale, ..StartRequest::default() };
    print!("agent> ");
    std::io::stdout().flush()?;
    let mut activities = client.start_conversation_with_request(start);
    while let Some(activity) = activities.try_next().await? {
        print_activity(&activity);
    }

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    loop {
        print!("\nuser> ");
        std::io::stdout().flush()?;
        let Some(question) = lines.next_line().await? else { break };
        if question.trim().is_empty() {
            continue;
        }
        let mut activities = client.ask_question(question, None);
        while let Some(activity) = activities.try_next().await? {
            print_activity(&activity);
        }
    }
    Ok(())
}

/// Mirrors `ChatConsoleService.PrintActivity` in the upstream samples.
fn print_activity(activity: &copilotstudio_client::Activity) {
    match activity.r#type {
        ActivityType::Message => {
            let text = activity.text.as_deref().unwrap_or("");
            if activity.text_format.as_deref() == Some(text_format_types::MARKDOWN) {
                println!("{text}");
                if let Some(actions) = activity.suggested_actions.as_ref().and_then(|s| s.actions.as_deref()) {
                    println!("Suggested actions:");
                    for action in actions {
                        println!("  - {}", action.text.as_deref().unwrap_or(&action.title));
                    }
                }
            } else {
                println!("{text}");
            }
        }
        ActivityType::Typing => {
            // Streamed chunks carry incremental text; show progress without echoing partial text.
            print!(".");
            let _ = std::io::stdout().flush();
        }
        ActivityType::Event => print!("+"),
        ref other => println!("Activity type: [{other}]"),
    }
}
