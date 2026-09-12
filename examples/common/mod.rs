//! Shared by the examples: Entra sign-in for a public-client app registration.
//!
//! Token acquisition is outside the library, exactly as upstream; the examples do what the
//! upstream console samples do with MSAL, here with the Microsoft identity platform **device
//! code** flow (no browser redirect needed on the machine running the example).
//! <https://learn.microsoft.com/entra/identity-platform/v2-oauth2-device-code>

use std::time::Duration;

/// Sign in with the device-code flow and return the access token for `scope`.
///
/// Reads `TENANT_ID`, `APP_CLIENT_ID` and optionally `AUTHORITY` (default
/// `https://login.microsoftonline.com`). The "open this URL and enter this code" message goes to
/// stderr so stdout stays clean for callers that capture the token.
pub async fn device_code_login(scope: &str) -> Result<String, Box<dyn std::error::Error>> {
    let tenant = std::env::var("TENANT_ID")
        .map_err(|_| "set COPILOTSTUDIO_TOKEN, or TENANT_ID + APP_CLIENT_ID for device-code login")?;
    let client_id = std::env::var("APP_CLIENT_ID").map_err(|_| "APP_CLIENT_ID is required for device-code login")?;
    let authority = std::env::var("AUTHORITY").unwrap_or_else(|_| "https://login.microsoftonline.com".to_owned());
    let http = reqwest::Client::new();

    let form = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("client_id", &client_id)
        .append_pair("scope", scope)
        .finish();
    let response = http
        .post(format!("{authority}/{tenant}/oauth2/v2.0/devicecode"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(form)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(format!(
            "device code request failed: {} {}",
            response.status(),
            response.text().await.unwrap_or_default()
        )
        .into());
    }
    let device: serde_json::Value = response.json().await?;
    eprintln!("{}", device["message"].as_str().unwrap_or("Complete the sign-in in your browser."));
    let device_code = device["device_code"].as_str().ok_or("no device_code in response")?.to_owned();
    let mut interval = Duration::from_secs(device["interval"].as_u64().unwrap_or(5));

    loop {
        tokio::time::sleep(interval).await;
        let form = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("grant_type", "urn:ietf:params:oauth:grant-type:device_code")
            .append_pair("client_id", &client_id)
            .append_pair("device_code", &device_code)
            .finish();
        let body: serde_json::Value = http
            .post(format!("{authority}/{tenant}/oauth2/v2.0/token"))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(form)
            .send()
            .await?
            .json()
            .await?;
        if let Some(token) = body["access_token"].as_str() {
            return Ok(token.to_owned());
        }
        match body["error"].as_str() {
            Some("authorization_pending") => continue,
            Some("slow_down") => interval += Duration::from_secs(5),
            Some(other) => return Err(format!("{other}: {}", body["error_description"].as_str().unwrap_or("")).into()),
            None => return Err(format!("unexpected token response: {body}").into()),
        }
    }
}
