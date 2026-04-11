/// GCP Confidential Space attestation helpers.
///
/// `fetch_jwt` talks to the local TEE launcher socket and returns an OIDC token.
/// Returns `None` when the socket is absent (dev / non-TEE environment).
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

const TEE_SOCKET: &str = "/run/container_launcher/teeserver.sock";

// ── JWT from TEE socket ───────────────────────────────────────────────────────

/// Fetch an OIDC JWT from the GCP Confidential Space launcher socket.
///
/// Returns `Ok(None)` when not running inside a Confidential Space (no socket).
/// Returns `Err` on communication or parse failures.
pub async fn fetch_jwt(audience: &str, nonces: Vec<String>) -> Result<Option<String>, String> {
    if !std::path::Path::new(TEE_SOCKET).exists() {
        return Ok(None);
    }

    let body = serde_json::json!({
        "audience": audience,
        "token_type": "OIDC",
        "nonces": nonces
    })
    .to_string();

    let http_req = format!(
        "POST /v1/token HTTP/1.0\r\n\
         Host: localhost\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         \r\n{}",
        body.len(),
        body
    );

    let mut stream = UnixStream::connect(TEE_SOCKET)
        .await
        .map_err(|e| format!("Failed to connect to TEE socket: {e}"))?;

    stream
        .write_all(http_req.as_bytes())
        .await
        .map_err(|e| format!("Failed to write to TEE socket: {e}"))?;

    // Signal EOF on the write side so the server knows the request is complete.
    stream
        .shutdown()
        .await
        .map_err(|e| format!("Failed to shutdown write half: {e}"))?;

    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .await
        .map_err(|e| format!("Failed to read TEE response: {e}"))?;

    let response = String::from_utf8_lossy(&raw);

    // Body starts after the blank line that separates HTTP headers from body.
    let body_start = response
        .find("\r\n\r\n")
        .ok_or_else(|| "Malformed HTTP response from TEE socket".to_string())?
        + 4;

    let jwt = response[body_start..].trim().to_string();

    if jwt.is_empty() {
        return Err("Empty JWT from TEE socket".to_string());
    }

    Ok(Some(jwt))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_jwt_returns_none_outside_tee() {
        // In a normal dev environment the TEE socket does not exist.
        let result = fetch_jwt("https://sts.googleapis.com", vec![]).await;
        assert!(result.is_ok());
        assert!(
            result.unwrap().is_none(),
            "should return None outside Confidential Space"
        );
    }

}
