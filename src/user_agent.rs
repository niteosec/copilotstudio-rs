//! The `User-Agent` sent on every request.
//!
//! Mirrors `UserAgentHelper` in all three clients: `{ClientName}.agents-sdk-{lang}/{version}`
//! followed by runtime and OS product tokens.

/// The client product name shared by every official client.
pub const CLIENT_NAME: &str = "CopilotStudioClient";

/// The `User-Agent` string: `CopilotStudioClient.agents-sdk-rust/{crate version} rust/{edition} {os}/{arch}`.
///
/// The runtime token is the compile-time edition (Rust exposes no runtime version); the OS token
/// follows the JS client's `{platform}-{arch}` shape.
pub fn user_agent() -> String {
    format!(
        "{CLIENT_NAME}.agents-sdk-rust/{} rust/2024 {}/{}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn has_upstream_shape() {
        let ua = super::user_agent();
        assert!(ua.starts_with("CopilotStudioClient.agents-sdk-rust/"));
        assert_eq!(ua.split(' ').count(), 3);
    }
}
