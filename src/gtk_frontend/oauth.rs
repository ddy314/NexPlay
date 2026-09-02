use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

pub struct OAuthCallback {
    pub code: String,
    pub state: String,
}

pub fn open_default_browser(uri: &str) -> Result<(), String> {
    gio::AppInfo::launch_default_for_uri(uri, None::<&gio::AppLaunchContext>)
        .map_err(|error| error.to_string())
}

/// Bind the configured loopback redirect and return a one-shot result channel.
/// The callback response is deliberately generic and never contains the
/// authorization code, token, or any client secret.
pub fn bind_loopback(
    redirect_uri: &str,
    expected_state: &str,
) -> Result<mpsc::Receiver<Result<OAuthCallback, String>>, String> {
    let uri = reqwest::Url::parse(redirect_uri).map_err(|error| error.to_string())?;
    if uri.scheme() != "http" {
        return Err("Bangumi OAuth redirect must use http on loopback".to_string());
    }
    if expected_state.trim().is_empty() {
        return Err("Bangumi OAuth state is empty".to_string());
    }
    let host = uri
        .host_str()
        .ok_or_else(|| "Bangumi redirect URI has no host".to_string())?;
    if host != "127.0.0.1" && host != "localhost" {
        return Err("Bangumi OAuth redirect must use a loopback host".to_string());
    }
    let port = uri
        .port_or_known_default()
        .ok_or_else(|| "Bangumi redirect URI has no port".to_string())?;
    let listener = TcpListener::bind((host, port))
        .map_err(|error| format!("cannot bind Bangumi OAuth callback on {host}:{port}: {error}"))?;
    let expected_state = expected_state.to_string();
    let (sender, receiver) = mpsc::sync_channel(1);
    let _ = thread::Builder::new()
        .name("nexplay-bangumi-oauth-callback".to_string())
        .spawn(move || {
            let result = accept_callback(listener, &expected_state);
            let _ = sender.send(result);
        });
    Ok(receiver)
}

fn accept_callback(listener: TcpListener, expected_state: &str) -> Result<OAuthCallback, String> {
    let (mut stream, _) = listener
        .accept()
        .map_err(|error| format!("Bangumi OAuth callback failed: {error}"))?;
    let mut buffer = [0_u8; 8192];
    let read = stream
        .read(&mut buffer)
        .map_err(|error| format!("Bangumi OAuth callback read failed: {error}"))?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let target = request
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("GET "))
        .and_then(|line| line.split_whitespace().next())
        .ok_or_else(|| "Bangumi OAuth callback was not a GET request".to_string())?;
    let callback_url = reqwest::Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|error| format!("invalid Bangumi OAuth callback URL: {error}"))?;
    let params = callback_url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<std::collections::HashMap<_, _>>();
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<h1>NexPlay</h1><p>授权完成，可以返回应用。</p>";
    let _ = stream.write_all(response.as_bytes());

    if let Some(error) = params.get("error") {
        return Err(format!("Bangumi OAuth was rejected: {error}"));
    }
    let code = params
        .get("code")
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .ok_or_else(|| {
            "Bangumi OAuth callback did not include an authorization code".to_string()
        })?;
    let state = params
        .get("state")
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .ok_or_else(|| "Bangumi OAuth callback did not include state".to_string())?;
    if state != expected_state {
        return Err("Bangumi OAuth state verification failed".to_string());
    }
    Ok(OAuthCallback { code, state })
}
