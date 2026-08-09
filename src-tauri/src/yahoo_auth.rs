//! Yahoo Finance session handshake for endpoints that require a crumb token.
//!
//! Yahoo's v7 `/finance/quote` endpoint (used by the Research Watchlist
//! snapshot fetch) started rejecting unauthenticated requests with HTTP 401
//! (see #789). A request now needs a session cookie plus a crumb token
//! derived from that cookie. The v8 `/finance/chart` endpoint used by the
//! main price-refresh path is unaffected and needs no change.
//!
//! The handshake takes two extra HTTP round trips, so the resulting crumb is
//! cached process-wide and only refreshed when a caller reports it stopped
//! working (crumbs can expire independently of any TTL we could predict).

use reqwest::Client;
use tokio::sync::RwLock;

use crate::config::USER_AGENT;

/// Session credentials required by Yahoo Finance's v7 quote endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YahooCrumb {
    pub crumb: String,
    pub cookie: String,
}

static CRUMB_CACHE: RwLock<Option<YahooCrumb>> = RwLock::const_new(None);

/// Returns the cached crumb, fetching a fresh one on first use.
pub async fn get_yahoo_crumb(client: &Client) -> Result<YahooCrumb, String> {
    if let Some(cached) = CRUMB_CACHE.read().await.clone() {
        return Ok(cached);
    }
    refresh_yahoo_crumb(client).await
}

/// Discards any cached crumb and fetches a fresh one. Call this when a
/// request made with the cached crumb comes back 401, then retry once.
pub async fn refresh_yahoo_crumb(client: &Client) -> Result<YahooCrumb, String> {
    let fresh = fetch_yahoo_crumb(
        client,
        crate::config::YAHOO_COOKIE_URL,
        crate::config::YAHOO_CRUMB_URL,
    )
    .await?;
    *CRUMB_CACHE.write().await = Some(fresh.clone());
    Ok(fresh)
}

/// Drops the cached crumb without fetching a replacement. Exposed so a
/// caller that gets 401 with a stale crumb can invalidate before deciding
/// whether to retry.
pub async fn invalidate_yahoo_crumb() {
    *CRUMB_CACHE.write().await = None;
}

/// Performs the two-step handshake: fetch a session cookie, then exchange it
/// for a crumb. `cookie_url`/`crumb_url` are parameterized so tests can point
/// this at a mock server instead of the real Yahoo Finance hosts.
async fn fetch_yahoo_crumb(
    client: &Client,
    cookie_url: &str,
    crumb_url: &str,
) -> Result<YahooCrumb, String> {
    let cookie_response = client
        .get(cookie_url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Failed to establish Yahoo Finance session: {}", e))?;

    let cookie = cookie_response
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        // Each Set-Cookie header is "name=value; Path=/; ...";  only the
        // "name=value" part before the first `;` belongs in a Cookie header.
        .map(|raw| raw.split(';').next().unwrap_or(raw).trim().to_string())
        .collect::<Vec<_>>()
        .join("; ");

    if cookie.is_empty() {
        return Err("Yahoo Finance did not return a session cookie".to_string());
    }

    let crumb_response = client
        .get(crumb_url)
        .header("User-Agent", USER_AGENT)
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Yahoo Finance crumb: {}", e))?;

    if !crumb_response.status().is_success() {
        return Err(format!(
            "HTTP {} fetching Yahoo Finance crumb",
            crumb_response.status()
        ));
    }

    let crumb = crumb_response
        .text()
        .await
        .map_err(|e| format!("Failed to read Yahoo Finance crumb response: {}", e))?
        .trim()
        .to_string();

    if crumb.is_empty() {
        return Err("Yahoo Finance returned an empty crumb".to_string());
    }

    Ok(YahooCrumb { crumb, cookie })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client() -> Client {
        Client::new()
    }

    #[tokio::test]
    async fn fetch_yahoo_crumb_success_returns_cookie_and_crumb() {
        let mut server = mockito::Server::new_async().await;
        let _cookie_mock = server
            .mock("GET", "/cookie")
            .with_status(200)
            .with_header("set-cookie", "A3=abc123; Path=/; Domain=.yahoo.com; Secure")
            .with_body("")
            .create_async()
            .await;
        let _crumb_mock = server
            .mock("GET", "/crumb")
            .match_header("cookie", "A3=abc123")
            .with_status(200)
            .with_body("a.crumbValue")
            .create_async()
            .await;

        let client = make_client();
        let cookie_url = format!("{}/cookie", server.url());
        let crumb_url = format!("{}/crumb", server.url());
        let result = fetch_yahoo_crumb(&client, &cookie_url, &crumb_url).await;

        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let creds = result.unwrap();
        assert_eq!(creds.crumb, "a.crumbValue");
        assert_eq!(creds.cookie, "A3=abc123");
    }

    #[tokio::test]
    async fn fetch_yahoo_crumb_missing_cookie_returns_error() {
        let mut server = mockito::Server::new_async().await;
        let _cookie_mock = server
            .mock("GET", "/cookie")
            .with_status(200)
            .with_body("")
            .create_async()
            .await;

        let client = make_client();
        let cookie_url = format!("{}/cookie", server.url());
        let result = fetch_yahoo_crumb(&client, &cookie_url, "https://example.invalid/crumb").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("session cookie"));
    }

    #[tokio::test]
    async fn fetch_yahoo_crumb_http_error_on_crumb_request_returns_error() {
        let mut server = mockito::Server::new_async().await;
        let _cookie_mock = server
            .mock("GET", "/cookie")
            .with_status(200)
            .with_header("set-cookie", "A3=abc123; Path=/")
            .with_body("")
            .create_async()
            .await;
        let _crumb_mock = server
            .mock("GET", "/crumb")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let client = make_client();
        let cookie_url = format!("{}/cookie", server.url());
        let crumb_url = format!("{}/crumb", server.url());
        let result = fetch_yahoo_crumb(&client, &cookie_url, &crumb_url).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("401"));
    }

    #[tokio::test]
    async fn fetch_yahoo_crumb_empty_crumb_body_returns_error() {
        let mut server = mockito::Server::new_async().await;
        let _cookie_mock = server
            .mock("GET", "/cookie")
            .with_status(200)
            .with_header("set-cookie", "A3=abc123; Path=/")
            .with_body("")
            .create_async()
            .await;
        let _crumb_mock = server
            .mock("GET", "/crumb")
            .with_status(200)
            .with_body("")
            .create_async()
            .await;

        let client = make_client();
        let cookie_url = format!("{}/cookie", server.url());
        let crumb_url = format!("{}/crumb", server.url());
        let result = fetch_yahoo_crumb(&client, &cookie_url, &crumb_url).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty crumb"));
    }

    #[tokio::test]
    async fn get_yahoo_crumb_caches_after_first_fetch() {
        // Uses the real process-wide cache, so run against real-looking mock
        // endpoints once, then confirm a second call doesn't need the mocks
        // (mockito would panic on an unexpected call once `.expect(1)` is hit).
        invalidate_yahoo_crumb().await;

        let mut server = mockito::Server::new_async().await;
        let cookie_mock = server
            .mock("GET", "/cookie-cache-test")
            .with_status(200)
            .with_header("set-cookie", "A3=cached; Path=/")
            .with_body("")
            .expect(1)
            .create_async()
            .await;
        let crumb_mock = server
            .mock("GET", "/crumb-cache-test")
            .with_status(200)
            .with_body("cached-crumb")
            .expect(1)
            .create_async()
            .await;

        let client = make_client();
        let cookie_url = format!("{}/cookie-cache-test", server.url());
        let crumb_url = format!("{}/crumb-cache-test", server.url());

        let first = fetch_yahoo_crumb(&client, &cookie_url, &crumb_url)
            .await
            .expect("first fetch should succeed");
        *CRUMB_CACHE.write().await = Some(first.clone());

        let second = get_yahoo_crumb(&client)
            .await
            .expect("second call should hit the cache, not the network");

        assert_eq!(first, second);
        cookie_mock.assert_async().await;
        crumb_mock.assert_async().await;

        invalidate_yahoo_crumb().await;
    }
}
