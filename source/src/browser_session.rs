//! Browser credentials stay in memory, except for a current-user DPAPI inbox envelope.
//! Every HTTP hop selects cookies again; redirect targets never inherit a Cookie header.
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{AUTHORIZATION, COOKIE, HOST, LOCATION, REFERER};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BrowserContext {
    pub cookies: Vec<BrowserCookie>,
    pub user_agent: String,
}
impl std::fmt::Debug for BrowserContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BrowserContext { credentials: [redacted] }")
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub host_only: bool,
    #[serde(default)]
    pub expires: Option<f64>,
}
pub fn network_url(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw).map_err(|_| "Invalid HTTP address.")?;
    if raw.len() > 16384
        || raw.chars().any(char::is_control)
        || !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Only HTTP(S) addresses without embedded credentials are supported.".into());
    }
    Ok(url)
}
impl BrowserContext {
    pub fn validate(&self) -> Result<(), String> {
        if self.cookies.len() > 256
            || self.user_agent.len() > 2048
            || self.user_agent.chars().any(char::is_control)
            || serde_json::to_vec(self)
                .map_err(|_| "Invalid browser session.")?
                .len()
                > 196608
        {
            return Err("Browser session exceeds the supported size.".into());
        }
        for c in &self.cookies {
            if c.name.is_empty()
                || c.name.len() > 256
                || c.value.len() > 16384
                || c.name
                    .bytes()
                    .any(|b| b <= 32 || b >= 127 || b"()<>@,;:\\\"/[]?={}".contains(&b))
                || c.value.bytes().any(|b| b < 32 || b == 127 || b == b';')
                || c.domain.is_empty()
                || c.domain.len() > 253
                || c.domain.starts_with('.')
                || c.domain
                    .bytes()
                    .any(|b| !(b.is_ascii_alphanumeric() || b".-:".contains(&b)))
                || !c.path.starts_with('/')
                || c.path.len() > 4096
                || c.path.chars().any(char::is_control)
                || c.expires.is_some_and(|v| !v.is_finite())
            {
                return Err("Invalid browser cookie metadata.".into());
            }
        }
        Ok(())
    }
    pub fn header(&self, url: &Url) -> String {
        let host = url.host_str().unwrap_or("").to_ascii_lowercase();
        let now = chrono::Utc::now().timestamp() as f64;
        let mut matching: Vec<_> = self
            .cookies
            .iter()
            .filter(|c| {
                let domain = c.domain.to_ascii_lowercase();
                (host == domain || (!c.host_only && host.ends_with(&format!(".{domain}"))))
                    && (!c.secure || url.scheme() == "https")
                    && c.expires.is_none_or(|expiry| expiry > now)
                    && (url.path() == c.path
                        || url
                            .path()
                            .strip_prefix(&c.path)
                            .is_some_and(|rest| c.path.ends_with('/') || rest.starts_with('/')))
            })
            .collect();
        matching.sort_by_key(|c| std::cmp::Reverse(c.path.len()));
        matching
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// The client MUST have redirects disabled. Headers are reconstructed at every hop.
pub fn send(
    client: &Client,
    builder: RequestBuilder,
    context: &BrowserContext,
) -> Result<Response, String> {
    let mut request = builder.build().map_err(|_| "Invalid download request.")?;
    for hop in 0..=10 {
        network_url(request.url().as_str())?;
        request.headers_mut().remove(COOKIE);
        let cookies = context.header(request.url());
        if !cookies.is_empty() {
            request.headers_mut().insert(
                COOKIE,
                cookies
                    .parse()
                    .map_err(|_| "Invalid browser cookie header.")?,
            );
        }
        let mut next = request
            .try_clone()
            .ok_or("Cannot repeat download request.")?;
        let response = client
            .execute(request)
            .map_err(|_| "The HTTP request failed or timed out.")?;
        if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
            return Ok(response);
        }
        if hop == 10 {
            return Err("Too many HTTP redirects.".into());
        }
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|v| v.to_str().ok())
            .ok_or("Invalid redirect location.")?;
        let target = response
            .url()
            .join(location)
            .map_err(|_| "Invalid redirect address.")?;
        network_url(target.as_str())?;
        if response.url().scheme() == "https" && target.scheme() != "https" {
            return Err("The server redirected a secure download to an insecure address.".into());
        }
        if response.url().origin() != target.origin() {
            next.headers_mut().remove(AUTHORIZATION);
            next.headers_mut().remove(HOST);
            // Match strict-origin-when-cross-origin without forwarding page query tokens.
            if let Some(value) = next.headers().get(REFERER).and_then(|h| h.to_str().ok()) {
                if let Ok(referer) = Url::parse(value) {
                    let origin = format!("{}/", referer.origin().ascii_serialization());
                    if let Ok(value) = origin.parse() {
                        next.headers_mut().insert(REFERER, value);
                    }
                }
            }
        }
        *next.url_mut() = target;
        request = next;
    }
    unreachable!()
}

#[cfg(windows)]
pub fn crypt(bytes: &[u8], decrypt: bool) -> Result<Vec<u8>, String> {
    #[repr(C)]
    struct Blob {
        len: u32,
        data: *mut u8,
    }
    #[link(name = "crypt32")]
    unsafe extern "system" {
        fn CryptProtectData(
            input: *const Blob,
            description: *const u16,
            entropy: *const Blob,
            reserved: *mut u8,
            prompt: *mut u8,
            flags: u32,
            output: *mut Blob,
        ) -> i32;
        fn CryptUnprotectData(
            input: *const Blob,
            description: *mut *mut u16,
            entropy: *const Blob,
            reserved: *mut u8,
            prompt: *mut u8,
            flags: u32,
            output: *mut Blob,
        ) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LocalFree(pointer: *mut u8) -> *mut u8;
    }
    if bytes.len() > 262144 {
        return Err("Browser session envelope is too large.".into());
    }
    let input = Blob {
        len: bytes.len() as u32,
        data: bytes.as_ptr() as *mut u8,
    };
    let mut output = Blob {
        len: 0,
        data: std::ptr::null_mut(),
    };
    unsafe {
        let ok = if decrypt {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut output,
            )
        } else {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut output,
            )
        };
        if ok == 0 {
            return Err("Windows could not protect or recover the browser session. Send the download again.".into());
        }
        let result = std::slice::from_raw_parts(output.data, output.len as usize).to_vec();
        LocalFree(output.data);
        Ok(result)
    }
}
#[cfg(not(windows))]
pub fn crypt(_: &[u8], _: bool) -> Result<Vec<u8>, String> {
    Err("Protected browser session handoff requires Windows.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> BrowserContext {
        serde_json::from_value(serde_json::json!({"cookies":[{"name":"session","value":"secret","domain":"example.com","path":"/files","secure":true,"host_only":true}]})).unwrap()
    }
    #[test]
    fn cookies_respect_host_path_tls_and_expiry() {
        let mut c = context();
        c.validate().unwrap();
        assert_eq!(
            c.header(&Url::parse("https://example.com/files/a").unwrap()),
            "session=secret"
        );
        for u in [
            "http://example.com/files/a",
            "https://cdn.example.com/files/a",
            "https://example.com/files-other",
            "https://elsewhere.com/files/a",
        ] {
            assert!(c.header(&Url::parse(u).unwrap()).is_empty());
        }
        c.cookies[0].expires = Some(1.0);
        assert!(
            c.header(&Url::parse("https://example.com/files/a").unwrap())
                .is_empty()
        );
    }
    #[test]
    fn rejects_header_injection_and_redacts_debug() {
        let mut c = context();
        assert!(!format!("{c:?}").contains("secret"));
        c.cookies[0].value = "a\r\nCookie: secret".into();
        assert!(c.validate().is_err());
    }
    #[test]
    #[cfg(windows)]
    fn inbox_envelope_is_encrypted_and_authenticated() {
        let raw = b"browser-secret-cookie";
        let protected = crypt(raw, false).unwrap();
        assert!(!protected.windows(raw.len()).any(|x| x == raw));
        assert_eq!(crypt(&protected, true).unwrap(), raw);
        let mut broken = protected;
        broken[0] ^= 1;
        assert!(crypt(&broken, true).is_err());
    }
    #[test]
    fn redirects_reselect_cookies_and_remove_credentials() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let worker = std::thread::spawn(move || {
            let mut seen = vec![];
            for hop in 0..2 {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut raw = vec![];
                let mut b = [0];
                while !raw.ends_with(b"\r\n\r\n") {
                    socket.read_exact(&mut b).unwrap();
                    raw.push(b[0]);
                }
                seen.push(String::from_utf8(raw).unwrap().to_lowercase());
                let reply = if hop == 0 {
                    format!(
                        "HTTP/1.1 302 Found\r\nLocation: http://localhost:{port}/outside\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".into()
                };
                socket.write_all(reply.as_bytes()).unwrap();
            }
            seen
        });
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        let mut c = context();
        c.cookies[0].domain = "127.0.0.1".into();
        c.cookies[0].secure = false;
        c.cookies[0].path = "/files".into();
        let result = send(
            &client,
            client
                .get(format!("http://127.0.0.1:{port}/files/start"))
                .basic_auth("name", Some("password")),
            &c,
        )
        .unwrap();
        assert_eq!(result.status(), 200);
        let seen = worker.join().unwrap();
        assert!(seen[0].contains("cookie: session=secret"));
        assert!(seen[0].contains("authorization:"));
        assert!(!seen[1].contains("cookie:"));
        assert!(!seen[1].contains("authorization:"));
    }
}
