//! Pure resolution of Direct-to-Engine URLs and token audiences from [`ConnectionSettings`].
//!
//! Mirrors `Discovery/PowerPlatformEnvironment.cs` (.NET, primary), `power_platform_environment.py`
//! and `powerPlatformEnvironment.ts` + `strategies/*.ts`. No I/O. See `docs/DESIGN.md` §3.1–3.2 for
//! the algorithm and the parity vectors.

use url::Url;

use crate::agent_type::AgentType;
use crate::connection_settings::ConnectionSettings;
use crate::error::SettingsError;
use crate::power_platform_cloud::PowerPlatformCloud;

/// The Power Platform API version every request carries.
pub const API_VERSION: &str = "2022-03-01-preview";

/// Base address substituted when none is known (upstream placeholder; never a real host).
const UNKNOWN_BASE_ADDRESS: &str = "api.unknown.powerplatform.com";

/// The D2E conversation URL for `settings`: `…/conversations` or `…/conversations/{id}`.
pub fn connection_url(settings: &ConnectionSettings, conversation_id: Option<&str>) -> Result<Url, SettingsError> {
    resolve_connection_url(settings, conversation_id, false)
}

/// The D2E subscribe URL for `settings`: `…/conversations/{id}/subscribe`.
pub fn subscribe_url(settings: &ConnectionSettings, conversation_id: &str) -> Result<Url, SettingsError> {
    resolve_connection_url(settings, Some(conversation_id), true)
}

/// The token audience (Entra scope) for `settings`, e.g. `https://api.powerplatform.com/.default`.
///
/// Mirrors `CopilotClient.ScopeFromSettings` / `scope_from_settings` / `ScopeHelper.getScopeFromSettings`.
pub fn scope_from_settings(settings: &ConnectionSettings) -> Result<String, SettingsError> {
    token_audience(Some(settings), PowerPlatformCloud::Unknown, None, None)
}

/// The token audience for a cloud, e.g. `https://api.powerplatform.com/.default`.
///
/// Mirrors `CopilotClient.ScopeFromCloud` / `scope_from_cloud`.
pub fn scope_from_cloud(cloud: PowerPlatformCloud) -> Result<String, SettingsError> {
    token_audience(None, cloud, None, None)
}

/// Full-signature audience resolution (`GetTokenAudience`).
///
/// `direct_connect_url` (explicit, else from `settings`) wins: the cloud is decoded from its host,
/// falling back to `settings.cloud` / `cloud` when the host is not a known Power Platform API host.
/// Otherwise the audience is `https://{endpoint suffix of the cloud}/.default`, where the cloud is
/// `settings.cloud` when set and not `Unknown`, else `cloud`.
pub fn token_audience(
    settings: Option<&ConnectionSettings>,
    cloud: PowerPlatformCloud,
    cloud_base_address: Option<&str>,
    direct_connect_url: Option<&str>,
) -> Result<String, SettingsError> {
    let direct =
        non_blank(direct_connect_url).or_else(|| settings.and_then(|s| non_blank(s.direct_connect_url.as_deref())));

    let Some(direct) = direct else {
        if cloud == PowerPlatformCloud::Other && non_blank(cloud_base_address).is_none() {
            return Err(SettingsError::CloudBaseAddressRequired);
        }
        if settings.is_none() && cloud == PowerPlatformCloud::Unknown {
            return Err(SettingsError::SettingsOrCloudRequired);
        }
        let mut cloud = cloud;
        // Python / JS default an unset cloud to `Prod` at construction; mirrored here so
        // `ConnectionSettings::new(env, schema)` resolves an audience.
        if let Some(c) = settings.map(|s| s.cloud.unwrap_or_default()) {
            if c != PowerPlatformCloud::Unknown {
                cloud = c;
            }
        }
        let mut base: Option<String> = non_blank(cloud_base_address).map(str::to_owned);
        if cloud == PowerPlatformCloud::Other {
            if base.as_deref().is_some_and(is_absolute_url) {
                // keep the explicit absolute base address
            } else if let Some(custom) =
                settings.and_then(|s| non_blank(s.custom_power_platform_cloud.as_deref())).filter(|c| is_well_formed(c))
            {
                base = Some(custom.to_owned());
            } else {
                return Err(SettingsError::CustomCloudOrBaseAddressRequired);
            }
            // Python normalises a scheme-qualified base address to its host.
            base = base.map(|b| host_of(&b).unwrap_or(b));
        }
        let base = base.unwrap_or_else(|| UNKNOWN_BASE_ADDRESS.to_owned());
        return Ok(format!("https://{}/.default", endpoint_suffix(cloud, &base)?));
    };

    let url = parse_absolute(direct).ok_or(SettingsError::InvalidDirectConnectUrl)?;
    let decoded = decode_cloud_from_url(&url);
    if decoded == PowerPlatformCloud::Unknown {
        let cloud_to_test = settings.and_then(|s| s.cloud).unwrap_or(cloud);
        if matches!(cloud_to_test, PowerPlatformCloud::Other | PowerPlatformCloud::Unknown) {
            return Err(SettingsError::UnableToResolveCloudFromDirectConnectUrl);
        }
        return Ok(format!("https://{}/.default", endpoint_suffix(cloud_to_test, "")?));
    }
    Ok(format!("https://{}/.default", endpoint_suffix(decoded, "")?))
}

/// The environment-scoped API host: `{prefix}.{suffix}.environment.{endpoint suffix}`.
///
/// `environment_id` is lower-cased and stripped of `-`; the last 2 hex chars (Prod / FirstRelease)
/// or 1 (every other cloud) become the middle label.
pub fn environment_endpoint(
    cloud: PowerPlatformCloud,
    environment_id: &str,
    cloud_base_address: Option<&str>,
) -> Result<String, SettingsError> {
    if cloud == PowerPlatformCloud::Other && non_blank(cloud_base_address).is_none() {
        return Err(SettingsError::CloudBaseAddressRequired);
    }
    let base = non_blank(cloud_base_address).unwrap_or(UNKNOWN_BASE_ADDRESS);
    let normalized = environment_id.to_lowercase().replace('-', "");
    let n = id_suffix_length(cloud);
    if normalized.len() < n {
        return Err(SettingsError::InvalidEnvironmentId);
    }
    let (prefix, suffix) = normalized.split_at(normalized.len() - n);
    Ok(format!("{prefix}.{suffix}.environment.{}", endpoint_suffix(cloud, base)?))
}

/// The Power Platform API host suffix for `cloud` (`Other` → `cloud_base_address`).
pub fn endpoint_suffix(cloud: PowerPlatformCloud, cloud_base_address: &str) -> Result<String, SettingsError> {
    use PowerPlatformCloud as C;
    let suffix = match cloud {
        C::Local => "api.powerplatform.localhost",
        C::Exp => "api.exp.powerplatform.com",
        C::Dev => "api.dev.powerplatform.com",
        C::Prv => "api.prv.powerplatform.com",
        C::Test => "api.test.powerplatform.com",
        C::Preprod => "api.preprod.powerplatform.com",
        C::FirstRelease | C::Prod => "api.powerplatform.com",
        C::GovFR | C::Gov => "api.gov.powerplatform.microsoft.us",
        C::High => "api.high.powerplatform.microsoft.us",
        C::DoD => "api.appsplatform.us",
        C::Mooncake => "api.powerplatform.partner.microsoftonline.cn",
        C::Ex => "api.powerplatform.eaglex.ic.gov",
        C::Rx => "api.powerplatform.microsoft.scloud",
        C::Other => cloud_base_address,
        C::Unknown => return Err(SettingsError::InvalidCloudCategory(cloud)),
    };
    if suffix.is_empty() {
        return Err(SettingsError::InvalidCloudCategory(cloud));
    }
    Ok(suffix.to_owned())
}

/// Decode the cloud from a direct-connect URL's host (`DecodeCloudFromURI`).
///
/// `api.gov.powerplatform.microsoft.us` decodes to `GovFR`; `Ex`, `Rx` and custom hosts are never
/// decoded and yield `Unknown`.
pub fn decode_cloud_from_url(url: &Url) -> PowerPlatformCloud {
    use PowerPlatformCloud as C;
    match url.host_str().map(str::to_ascii_lowercase).as_deref() {
        Some("api.powerplatform.localhost") => C::Local,
        Some("api.exp.powerplatform.com") => C::Exp,
        Some("api.dev.powerplatform.com") => C::Dev,
        Some("api.prv.powerplatform.com") => C::Prv,
        Some("api.test.powerplatform.com") => C::Test,
        Some("api.preprod.powerplatform.com") => C::Preprod,
        Some("api.powerplatform.com") => C::Prod,
        Some("api.gov.powerplatform.microsoft.us") => C::GovFR,
        Some("api.high.powerplatform.microsoft.us") => C::High,
        Some("api.appsplatform.us") => C::DoD,
        Some("api.powerplatform.partner.microsoftonline.cn") => C::Mooncake,
        _ => C::Unknown,
    }
}

/// `GetCopilotStudioConnectionUrl` with the defaults folded in
/// (`agentType = Published`, `cloud = Prod`, `cloudBaseAddress = null`, `directConnectUrl = null`).
fn resolve_connection_url(
    settings: &ConnectionSettings,
    conversation_id: Option<&str>,
    subscribe: bool,
) -> Result<Url, SettingsError> {
    let Some(direct) = non_blank(settings.direct_connect_url.as_deref()) else {
        let environment_id =
            non_blank(settings.environment_id.as_deref()).ok_or(SettingsError::EnvironmentIdRequired)?;
        let schema_name = non_blank(settings.schema_name.as_deref()).ok_or(SettingsError::AgentIdentifierRequired)?;

        let mut cloud = PowerPlatformCloud::Prod;
        if let Some(c) = settings.cloud {
            if c != PowerPlatformCloud::Unknown {
                cloud = c;
            }
        }
        let mut cloud_base_address: Option<String> = None;
        if cloud == PowerPlatformCloud::Other {
            match non_blank(settings.custom_power_platform_cloud.as_deref()).filter(|c| is_well_formed(c)) {
                // Python normalises a scheme-qualified base address to its host on the audience path;
                // applied here too so both derive the same host (see docs/DESIGN.md §6, D13).
                Some(custom) => cloud_base_address = Some(host_of(custom).unwrap_or_else(|| custom.to_owned())),
                None => return Err(SettingsError::CustomCloudOrBaseAddressRequired),
            }
        }
        let agent_type = settings.copilot_agent_type.unwrap_or_default();
        let host = environment_endpoint(cloud, environment_id, cloud_base_address.as_deref())?;
        return create_uri_standard(schema_name, &host, agent_type, conversation_id, subscribe);
    };

    let base = parse_absolute(direct).ok_or(SettingsError::InvalidDirectConnectUrl)?;
    Ok(create_uri_direct(base, conversation_id, subscribe))
}

/// `CreateUri(schemaName, host, agentType, conversationId, createSubscribeLink)`.
fn create_uri_standard(
    schema_name: &str,
    host: &str,
    agent_type: AgentType,
    conversation_id: Option<&str>,
    subscribe: bool,
) -> Result<Url, SettingsError> {
    let mut url = Url::parse(&format!("https://{host}")).map_err(|_| SettingsError::InvalidCustomPowerPlatformCloud)?;
    // The authority must survive parsing verbatim; anything else means the custom cloud carried a
    // path, userinfo or characters that do not form a host.
    let authority = match (url.host_str(), url.port()) {
        (Some(h), Some(p)) => format!("{h}:{p}"),
        (Some(h), None) => h.to_owned(),
        (None, _) => return Err(SettingsError::InvalidCustomPowerPlatformCloud),
    };
    if !authority.eq_ignore_ascii_case(host) || url.path() != "/" || url.query().is_some() {
        return Err(SettingsError::InvalidCustomPowerPlatformCloud);
    }
    {
        let mut segments = url.path_segments_mut().map_err(|_| SettingsError::InvalidCustomPowerPlatformCloud)?;
        segments.clear();
        segments
            .push("copilotstudio")
            .push(agent_type.path_segment())
            .push("authenticated")
            .push("bots")
            .push(schema_name)
            .push("conversations");
        push_conversation(&mut segments, conversation_id, subscribe);
    }
    url.set_query(Some(&format!("api-version={API_VERSION}")));
    Ok(url)
}

/// `CreateUri(baseaddress, conversationId, createSubscribeLink)` — direct-connect mode.
///
/// Strips trailing `/` or `\`, cuts the path at the first `/conversations`, then appends
/// `/conversations[/{id}[/subscribe]]`; the query becomes exactly `api-version=…`.
fn create_uri_direct(mut url: Url, conversation_id: Option<&str>, subscribe: bool) -> Url {
    let mut path = url.path().to_owned();
    while path.ends_with('/') || path.ends_with('\\') {
        path.pop();
    }
    if let Some(i) = path.find("/conversations") {
        path.truncate(i);
    }
    url.set_path(&path);
    url.set_fragment(None);
    if let Ok(mut segments) = url.path_segments_mut() {
        segments.pop_if_empty().push("conversations");
        push_conversation(&mut segments, conversation_id, subscribe);
    }
    url.set_query(Some(&format!("api-version={API_VERSION}")));
    url
}

fn push_conversation(segments: &mut url::PathSegmentsMut<'_>, conversation_id: Option<&str>, subscribe: bool) {
    if let Some(id) = non_blank(conversation_id) {
        segments.push(id);
        if subscribe {
            segments.push("subscribe");
        }
    }
}

fn id_suffix_length(cloud: PowerPlatformCloud) -> usize {
    match cloud {
        PowerPlatformCloud::FirstRelease | PowerPlatformCloud::Prod => 2,
        _ => 1,
    }
}

fn non_blank(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|s| !s.is_empty())
}

/// An absolute `http(s)` URL with a host (`Uri.IsWellFormedUriString(_, UriKind.Absolute)`).
fn parse_absolute(s: &str) -> Option<Url> {
    let url = Url::parse(s.trim()).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host_str().is_some()).then_some(url)
}

fn is_absolute_url(s: &str) -> bool {
    parse_absolute(s).is_some()
}

/// `Uri.IsWellFormedUriString(_, UriKind.RelativeOrAbsolute)`: absolute, or a bare host/authority
/// that would parse once given a scheme.
fn is_well_formed(s: &str) -> bool {
    !s.contains(char::is_whitespace)
        && (is_absolute_url(s) || Url::parse(&format!("https://{s}")).is_ok_and(|u| u.host_str().is_some()))
}

/// `host[:port]` of a scheme-qualified address; `None` when the value is not an absolute URL.
fn host_of(s: &str) -> Option<String> {
    let url = parse_absolute(s)?;
    let host = url.host_str()?;
    Some(match url.port() {
        Some(p) => format!("{host}:{p}"),
        None => host.to_owned(),
    })
}
