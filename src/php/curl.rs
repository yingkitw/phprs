//! cURL functions backed by `reqwest` (blocking client).
//!
//! Implements `curl_exec` request execution: GET by default, POST when
//! `CURLOPT_POSTFIELDS` is set, custom method via `CURLOPT_CUSTOMREQUEST`.

use std::time::Duration;

/// Execute an HTTP request and return `(body, status_code)`.
pub fn execute_request(
    url: &str,
    method: &str,
    body: &str,
    timeout_secs: u64,
) -> Result<(String, u16), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("curl: failed to build client: {e}"))?;

    let req = if !body.is_empty() {
        if method.is_empty() {
            client.post(url).body(body.to_string())
        } else {
            client.request(reqwest::Method::from_bytes(method.as_bytes())
                .unwrap_or(reqwest::Method::POST), url)
                .body(body.to_string())
        }
    } else if !method.is_empty() {
        client.request(reqwest::Method::from_bytes(method.as_bytes())
            .unwrap_or(reqwest::Method::GET), url)
    } else {
        client.get(url)
    };

    let resp = req.send().map_err(|e| format!("curl: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp.text().map_err(|e| format!("curl: {e}"))?;
    Ok((text, status))
}
