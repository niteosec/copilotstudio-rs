//! Caller-supplied bearer tokens.
//!
//! The reference client never acquires tokens: .NET takes a `Func<string, Task<string>>` keyed by
//! the request URL, Python and JS take a bearer string. [`TokenProvider`] is the former,
//! [`StaticToken`] the latter. MSAL / OBO exchange stays in the caller (see `docs/DESIGN.md` §4).

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::error::BoxError;

/// A boxed, `Send` future — the return type of [`TokenProvider::access_token`].
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Supplies the bearer token for each request.
///
/// Called once per HTTP request with the fully-resolved request URL (mirrors the .NET
/// `tokenProviderFunction`). Implementations decide whether to cache, refresh, or exchange.
pub trait TokenProvider: Send + Sync {
    /// Return the raw access token (without the `Bearer ` prefix) to send to `request_url`.
    fn access_token(&self, request_url: &str) -> BoxFuture<'_, Result<String, BoxError>>;
}

impl<T: TokenProvider + ?Sized> TokenProvider for Arc<T> {
    fn access_token(&self, request_url: &str) -> BoxFuture<'_, Result<String, BoxError>> {
        (**self).access_token(request_url)
    }
}

impl<T: TokenProvider + ?Sized> TokenProvider for Box<T> {
    fn access_token(&self, request_url: &str) -> BoxFuture<'_, Result<String, BoxError>> {
        (**self).access_token(request_url)
    }
}

/// A fixed bearer token — the Python / JS constructor shape.
///
/// `Debug` never prints the token.
#[derive(Clone)]
pub struct StaticToken(String);

impl StaticToken {
    /// Wrap an already-acquired access token.
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }
}

impl fmt::Debug for StaticToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StaticToken([REDACTED])")
    }
}

impl TokenProvider for StaticToken {
    fn access_token(&self, _request_url: &str) -> BoxFuture<'_, Result<String, BoxError>> {
        Box::pin(std::future::ready(Ok(self.0.clone())))
    }
}

/// Adapts an async closure `Fn(&str) -> impl Future<Output = Result<String, BoxError>>` into a
/// [`TokenProvider`].
pub struct TokenFn<F>(pub F);

impl<F, Fut> TokenProvider for TokenFn<F>
where
    F: Fn(String) -> Fut + Send + Sync,
    Fut: Future<Output = Result<String, BoxError>> + Send + 'static,
{
    fn access_token(&self, request_url: &str) -> BoxFuture<'_, Result<String, BoxError>> {
        Box::pin((self.0)(request_url.to_owned()))
    }
}

impl<F> fmt::Debug for TokenFn<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TokenFn(..)")
    }
}
