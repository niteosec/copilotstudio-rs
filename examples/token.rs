//! Sign in with the device-code flow and print the access token for the configured agent's
//! audience — and nothing else — on stdout, so it can be captured:
//!
//! ```text
//! export COPILOTSTUDIO_TOKEN=$(ENVIRONMENT_ID=… SCHEMA_NAME=… TENANT_ID=… APP_CLIENT_ID=… cargo run -q --example token)
//! cargo test --test live -- --ignored --nocapture
//! ```
//! The sign-in prompt goes to stderr. The token is a bearer credential: keep it in the environment
//! of the process that needs it and nowhere else.

mod common;

use copilotstudio_client::ConnectionSettings;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = ConnectionSettings::from_env()?;
    let scope = copilotstudio_client::scope_from_settings(&settings)?;
    eprintln!("requesting scope: {scope}");
    let token = common::device_code_login(&scope).await?;
    println!("{token}");
    Ok(())
}
