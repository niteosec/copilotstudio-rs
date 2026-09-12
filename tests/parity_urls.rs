//! Mirror-parity vectors for URL and token-audience resolution, transcribed from the upstream
//! test suites at the pinned tags (docs/DESIGN.md §11.1):
//! - .NET `CopilotClientTests.VerifyConnectionUrl` / `VerifyAgentScopeTest`
//! - JS `powerPlatformEnvironment.test.ts`, `publishedBotStrategy.test.ts`, `prebuiltBotStrategy.test.ts`
//! - Python `test_copilot_client.py` (direct-connect normalisation, subscribe links, cloud decode)

use copilotstudio_client::power_platform_environment::{decode_cloud_from_url, environment_endpoint, token_audience};
use copilotstudio_client::{
    AgentType, ConnectionSettings, PowerPlatformCloud, SettingsError, connection_url, scope_from_cloud,
    scope_from_settings, subscribe_url,
};
use url::Url;

const ENV: &str = "A47151CF-4F34-488F-B377-EBE84E17B478";
const API: &str = "api-version=2022-03-01-preview";

fn settings(cloud: PowerPlatformCloud, agent_type: AgentType, custom: &str) -> ConnectionSettings {
    let mut s = ConnectionSettings::new(ENV, "Bot01").cloud(cloud).agent_type(agent_type);
    if !custom.is_empty() {
        s = s.custom_power_platform_cloud(custom);
    }
    s
}

// ---- .NET VerifyConnectionUrl -----------------------------------------------------------------

#[test]
fn net_verify_connection_url_other_published_no_conversation() {
    let url = connection_url(&settings(PowerPlatformCloud::Other, AgentType::Published, "foo.api.com"), None).unwrap();
    assert_eq!(
        url.as_str(),
        format!(
            "https://a47151cf4f34488fb377ebe84e17b47.8.environment.foo.api.com/copilotstudio/dataverse-backed/authenticated/bots/Bot01/conversations?{API}"
        )
    );
}

#[test]
fn net_verify_connection_url_preprod_published() {
    let url = connection_url(&settings(PowerPlatformCloud::Preprod, AgentType::Published, ""), None).unwrap();
    assert_eq!(
        url.as_str(),
        format!(
            "https://a47151cf4f34488fb377ebe84e17b47.8.environment.api.preprod.powerplatform.com/copilotstudio/dataverse-backed/authenticated/bots/Bot01/conversations?{API}"
        )
    );
}

#[test]
fn net_verify_connection_url_prod_published() {
    let url = connection_url(&settings(PowerPlatformCloud::Prod, AgentType::Published, ""), None).unwrap();
    assert_eq!(
        url.as_str(),
        format!(
            "https://a47151cf4f34488fb377ebe84e17b4.78.environment.api.powerplatform.com/copilotstudio/dataverse-backed/authenticated/bots/Bot01/conversations?{API}"
        )
    );
}

#[test]
fn net_verify_connection_url_first_release_published() {
    let url = connection_url(&settings(PowerPlatformCloud::FirstRelease, AgentType::Published, ""), None).unwrap();
    assert_eq!(
        url.as_str(),
        format!(
            "https://a47151cf4f34488fb377ebe84e17b4.78.environment.api.powerplatform.com/copilotstudio/dataverse-backed/authenticated/bots/Bot01/conversations?{API}"
        )
    );
}

#[test]
fn net_verify_connection_url_first_release_with_conversation() {
    let url =
        connection_url(&settings(PowerPlatformCloud::FirstRelease, AgentType::Published, ""), Some("1234")).unwrap();
    assert_eq!(
        url.as_str(),
        format!(
            "https://a47151cf4f34488fb377ebe84e17b4.78.environment.api.powerplatform.com/copilotstudio/dataverse-backed/authenticated/bots/Bot01/conversations/1234?{API}"
        )
    );
}

#[test]
fn net_verify_connection_url_prod_prebuilt_with_conversation() {
    let url = connection_url(&settings(PowerPlatformCloud::Prod, AgentType::Prebuilt, ""), Some("1234")).unwrap();
    assert_eq!(
        url.as_str(),
        format!(
            "https://a47151cf4f34488fb377ebe84e17b4.78.environment.api.powerplatform.com/copilotstudio/prebuilt/authenticated/bots/Bot01/conversations/1234?{API}"
        )
    );
}

#[test]
fn net_verify_connection_url_other_with_malformed_custom_cloud_errors() {
    let err = connection_url(&settings(PowerPlatformCloud::Other, AgentType::Prebuilt, "Blah+1_ Blah"), Some("1234"))
        .unwrap_err();
    assert_eq!(err, SettingsError::CustomCloudOrBaseAddressRequired);
}

// ---- .NET VerifyAgentScopeTest ----------------------------------------------------------------

#[test]
fn net_verify_agent_scope() {
    let cases = [
        (PowerPlatformCloud::Prod, "", "https://api.powerplatform.com/.default"),
        (PowerPlatformCloud::Preprod, "", "https://api.preprod.powerplatform.com/.default"),
        (PowerPlatformCloud::Mooncake, "", "https://api.powerplatform.partner.microsoftonline.cn/.default"),
        (PowerPlatformCloud::FirstRelease, "", "https://api.powerplatform.com/.default"),
        (PowerPlatformCloud::Other, "fido.com", "https://fido.com/.default"),
    ];
    for (cloud, custom, expected) in cases {
        let scope = scope_from_settings(&settings(cloud, AgentType::Published, custom)).unwrap();
        assert_eq!(scope, expected, "cloud {cloud}");
    }
    let err = scope_from_settings(&settings(PowerPlatformCloud::Unknown, AgentType::Published, "")).unwrap_err();
    assert_eq!(err, SettingsError::InvalidCloudCategory(PowerPlatformCloud::Unknown));
}

#[test]
fn scope_from_cloud_matches_settings_path() {
    assert_eq!(
        scope_from_cloud(PowerPlatformCloud::Gov).unwrap(),
        "https://api.gov.powerplatform.microsoft.us/.default"
    );
    assert_eq!(scope_from_cloud(PowerPlatformCloud::DoD).unwrap(), "https://api.appsplatform.us/.default");
    assert_eq!(scope_from_cloud(PowerPlatformCloud::Unknown).unwrap_err(), SettingsError::SettingsOrCloudRequired);
    assert_eq!(scope_from_cloud(PowerPlatformCloud::Other).unwrap_err(), SettingsError::CloudBaseAddressRequired);
}

// ---- JS strategy + powerPlatformEnvironment tests -----------------------------------------------

const RESERVED_SCHEMA: &str = "bot/name?with#reserved characters";
const RESERVED_CONVERSATION: &str = "conversation/id?with#reserved characters";
const ENCODED_SCHEMA: &str = "bot%2Fname%3Fwith%23reserved%20characters";
const ENCODED_CONVERSATION: &str = "conversation%2Fid%3Fwith%23reserved%20characters";

#[test]
fn js_published_strategy_encodes_schema_and_conversation_as_path_segments() {
    let s = ConnectionSettings::new(ENV, RESERVED_SCHEMA)
        .cloud(PowerPlatformCloud::Other)
        .custom_power_platform_cloud("api.example.test");
    let url = connection_url(&s, Some(RESERVED_CONVERSATION)).unwrap();
    assert_eq!(
        url.path(),
        format!(
            "/copilotstudio/dataverse-backed/authenticated/bots/{ENCODED_SCHEMA}/conversations/{ENCODED_CONVERSATION}"
        )
    );
    assert_eq!(url.query(), Some(API));
    assert_eq!(url.host_str(), Some("a47151cf4f34488fb377ebe84e17b47.8.environment.api.example.test"));
}

#[test]
fn js_prebuilt_strategy_encodes_schema_and_conversation_as_path_segments() {
    let s = ConnectionSettings::new(ENV, RESERVED_SCHEMA)
        .cloud(PowerPlatformCloud::Other)
        .custom_power_platform_cloud("api.example.test")
        .agent_type(AgentType::Prebuilt);
    let url = connection_url(&s, Some(RESERVED_CONVERSATION)).unwrap();
    assert_eq!(
        url.path(),
        format!("/copilotstudio/prebuilt/authenticated/bots/{ENCODED_SCHEMA}/conversations/{ENCODED_CONVERSATION}")
    );
}

#[test]
fn js_direct_connection_url_encodes_conversation_id() {
    let s = ConnectionSettings::direct("https://api.example.test/copilotstudio/bots/test-bot");
    assert_eq!(
        connection_url(&s, Some(RESERVED_CONVERSATION)).unwrap().as_str(),
        format!("https://api.example.test/copilotstudio/bots/test-bot/conversations/{ENCODED_CONVERSATION}?{API}")
    );
}

#[test]
fn js_direct_connection_subscribe_url() {
    let s = ConnectionSettings::direct("https://api.example.test/copilotstudio/bots/test-bot");
    assert_eq!(
        subscribe_url(&s, RESERVED_CONVERSATION).unwrap().as_str(),
        format!(
            "https://api.example.test/copilotstudio/bots/test-bot/conversations/{ENCODED_CONVERSATION}/subscribe?{API}"
        )
    );
}

#[test]
fn js_token_audience_from_direct_connect_url() {
    let audience = token_audience(
        None,
        PowerPlatformCloud::Unknown,
        None,
        Some("https://api.powerplatform.com/copilotstudio/bots/test-bot"),
    )
    .unwrap();
    assert_eq!(audience, "https://api.powerplatform.com/.default");
}

// ---- Python test_copilot_client.py ---------------------------------------------------------------

const DIRECT: &str = "https://api.powerplatform.com/copilotstudio/dataverse-backed/authenticated/bots/test-bot";

#[test]
fn py_direct_connect_without_conversation() {
    let url = connection_url(&ConnectionSettings::direct(DIRECT), None).unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("api.powerplatform.com"));
    assert_eq!(url.as_str(), format!("{DIRECT}/conversations?{API}"));
}

#[test]
fn py_direct_connect_with_conversation() {
    let url = connection_url(&ConnectionSettings::direct(DIRECT), Some("conv-123")).unwrap();
    assert_eq!(url.as_str(), format!("{DIRECT}/conversations/conv-123?{API}"));
}

#[test]
fn py_subscribe_link_standard_mode() {
    let s = ConnectionSettings::new("test-env", "test-agent").cloud(PowerPlatformCloud::Prod);
    let url = subscribe_url(&s, "conv-456").unwrap();
    assert!(url.path().ends_with("/conversations/conv-456/subscribe"), "{url}");
    assert_eq!(url.host_str(), Some("teste.nv.environment.api.powerplatform.com"));
}

#[test]
fn py_subscribe_link_direct_connect() {
    let url = subscribe_url(&ConnectionSettings::direct(DIRECT), "conv-789").unwrap();
    assert_eq!(url.as_str(), format!("{DIRECT}/conversations/conv-789/subscribe?{API}"));
}

#[test]
fn py_direct_connect_path_normalisation_strips_existing_conversations_segment() {
    let s = ConnectionSettings::direct(format!("{DIRECT}/conversations"));
    let url = connection_url(&s, Some("conv-abc")).unwrap();
    assert!(!url.as_str().contains("/conversations/conversations"));
    assert_eq!(url.as_str(), format!("{DIRECT}/conversations/conv-abc?{API}"));
}

#[test]
fn direct_connect_strips_trailing_slash_and_replaces_query_and_fragment() {
    let s = ConnectionSettings::direct("https://island.example/base/?foo=bar#frag");
    assert_eq!(connection_url(&s, None).unwrap().as_str(), format!("https://island.example/base/conversations?{API}"));
    let s = ConnectionSettings::direct("https://island.example");
    assert_eq!(
        connection_url(&s, Some("c")).unwrap().as_str(),
        format!("https://island.example/conversations/c?{API}")
    );
}

#[test]
fn py_decode_cloud_from_uri() {
    let decode = |s: &str| decode_cloud_from_url(&Url::parse(s).unwrap());
    assert_eq!(decode("https://api.powerplatform.com/some/path"), PowerPlatformCloud::Prod);
    assert_eq!(decode("https://api.gov.powerplatform.microsoft.us/some/path"), PowerPlatformCloud::GovFR);
    assert_eq!(decode("https://custom.domain.com/some/path"), PowerPlatformCloud::Unknown);
    assert_eq!(decode("https://API.PREPROD.POWERPLATFORM.COM/"), PowerPlatformCloud::Preprod);
}

#[test]
fn py_token_audience_with_direct_connect_settings() {
    assert_eq!(
        scope_from_settings(&ConnectionSettings::direct(DIRECT)).unwrap(),
        "https://api.powerplatform.com/.default"
    );
}

#[test]
fn py_scope_from_settings_is_https_default() {
    let scope = scope_from_settings(&ConnectionSettings::new("env-id", "agent-id")).unwrap();
    assert!(scope.starts_with("https://") && scope.ends_with("/.default"));
}

// ---- Resolution rules -------------------------------------------------------------------------

#[test]
fn unknown_host_direct_url_falls_back_to_settings_cloud_or_errors() {
    let s = ConnectionSettings::direct("https://island.example/bot").cloud(PowerPlatformCloud::Preprod);
    assert_eq!(scope_from_settings(&s).unwrap(), "https://api.preprod.powerplatform.com/.default");
    let s = ConnectionSettings::direct("https://island.example/bot");
    assert_eq!(scope_from_settings(&s).unwrap_err(), SettingsError::UnableToResolveCloudFromDirectConnectUrl);
    let s = ConnectionSettings::direct("https://island.example/bot").cloud(PowerPlatformCloud::Other);
    assert_eq!(scope_from_settings(&s).unwrap_err(), SettingsError::UnableToResolveCloudFromDirectConnectUrl);
}

#[test]
fn missing_settings_are_reported_with_upstream_messages() {
    assert_eq!(connection_url(&ConnectionSettings::default(), None).unwrap_err(), SettingsError::EnvironmentIdRequired);
    let s = ConnectionSettings { environment_id: Some(ENV.into()), ..Default::default() };
    assert_eq!(connection_url(&s, None).unwrap_err(), SettingsError::AgentIdentifierRequired);
    let s = ConnectionSettings::new(ENV, "Bot01").cloud(PowerPlatformCloud::Other);
    assert_eq!(connection_url(&s, None).unwrap_err(), SettingsError::CustomCloudOrBaseAddressRequired);
    assert_eq!(
        connection_url(&ConnectionSettings::direct("not a url"), None).unwrap_err(),
        SettingsError::InvalidDirectConnectUrl
    );
    assert_eq!(
        connection_url(&ConnectionSettings::direct("/relative/path"), None).unwrap_err(),
        SettingsError::InvalidDirectConnectUrl
    );
    assert_eq!(SettingsError::EnvironmentIdRequired.to_string(), "EnvironmentId must be provided");
}

#[test]
fn unknown_cloud_in_settings_resolves_to_prod() {
    let s = ConnectionSettings::new(ENV, "Bot01").cloud(PowerPlatformCloud::Unknown);
    assert_eq!(
        connection_url(&s, None).unwrap().host_str(),
        Some("a47151cf4f34488fb377ebe84e17b4.78.environment.api.powerplatform.com")
    );
}

#[test]
fn custom_cloud_with_scheme_is_reduced_to_its_host_in_both_url_and_audience() {
    let s = ConnectionSettings::new(ENV, "Bot01")
        .cloud(PowerPlatformCloud::Other)
        .custom_power_platform_cloud("https://api.contoso.example:8443");
    assert_eq!(
        connection_url(&s, None).unwrap().host_str(),
        Some("a47151cf4f34488fb377ebe84e17b47.8.environment.api.contoso.example")
    );
    assert_eq!(connection_url(&s, None).unwrap().port(), Some(8443));
    assert_eq!(scope_from_settings(&s).unwrap(), "https://api.contoso.example:8443/.default");
}

#[test]
fn environment_endpoint_suffix_lengths() {
    assert_eq!(
        environment_endpoint(PowerPlatformCloud::Prod, ENV, None).unwrap(),
        "a47151cf4f34488fb377ebe84e17b4.78.environment.api.powerplatform.com"
    );
    assert_eq!(
        environment_endpoint(PowerPlatformCloud::Gov, ENV, None).unwrap(),
        "a47151cf4f34488fb377ebe84e17b47.8.environment.api.gov.powerplatform.microsoft.us"
    );
    assert_eq!(
        environment_endpoint(PowerPlatformCloud::Prod, "a", None).unwrap_err(),
        SettingsError::InvalidEnvironmentId
    );
    assert_eq!(
        environment_endpoint(PowerPlatformCloud::Other, ENV, None).unwrap_err(),
        SettingsError::CloudBaseAddressRequired
    );
}
