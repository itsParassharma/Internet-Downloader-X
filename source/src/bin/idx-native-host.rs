//! Small native-messaging bridge. It only hands an explicit HTTP(S) URL to IDX.
//! Standard output is reserved for length-prefixed protocol replies.

use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use url::Url;
#[path = "../browser_media.rs"]
mod browser_media;
#[allow(dead_code)]
#[path = "../browser_session.rs"]
mod browser_session;
#[allow(dead_code)]
#[path = "../streaming.rs"]
mod streaming;

const MAX_MESSAGE_BYTES: usize = 256 * 1024;
const MAX_URL_BYTES: usize = 16 * 1024;

#[derive(Deserialize)]
#[serde(untagged)]
enum Request {
    Ping(PingRequest),
    Download(DownloadRequest),
    Inspect(InspectRequest),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PingRequest {
    #[serde(rename = "action")]
    _action: PingAction,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum PingAction {
    Ping,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DownloadRequest {
    url: String,
    #[serde(default)]
    automatic: bool,
    #[serde(default)]
    parent_page: String,
    #[serde(default)]
    filename: String,
    #[serde(default)]
    referer: String,
    #[serde(default)]
    browser: browser_session::BrowserContext,
    #[serde(default)]
    stream: streaming::StreamOptions,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectRequest {
    action: InspectAction,
    url: String,
    kind: String,
    #[serde(default)]
    referer: String,
    #[serde(default)]
    browser: browser_session::BrowserContext,
}
#[derive(Deserialize)]
enum InspectAction {
    #[serde(rename = "media_info")]
    MediaInfo,
}
fn inspect(request: InspectRequest) -> Result<Value, String> {
    let _ = request.action;
    validated_url(&request.url).map_err(str::to_owned)?;
    request.browser.validate()?;
    if !matches!(request.kind.as_str(), "hls" | "dash") {
        return Err("Invalid media format.".into());
    }
    if !request.referer.is_empty() {
        validated_url(&request.referer).map_err(str::to_owned)?;
    }
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|_| "Could not create media request.")?;
    let choices = streaming::qualities(&request.url, &request.kind, &mut |url| {
        let mut builder = client.get(url).header("Accept-Encoding", "identity");
        if !request.referer.is_empty() {
            builder = builder.header("Referer", &request.referer);
        }
        if !request.browser.user_agent.is_empty() {
            builder = builder.header("User-Agent", &request.browser.user_agent);
        }
        let mut response = browser_session::send(&client, builder, &request.browser)?;
        if !response.status().is_success() {
            return Err(format!(
                "The media server returned HTTP {}.",
                response.status().as_u16()
            ));
        }
        let base = response.url().to_string();
        let mut data = vec![];
        response
            .by_ref()
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut data)
            .map_err(|_| "Could not read media manifest.")?;
        if data.len() > 4 * 1024 * 1024 {
            return Err("Media manifest is too large.".into());
        }
        Ok((
            base,
            String::from_utf8(data).map_err(|_| "Media manifest is not UTF-8.")?,
        ))
    })?;
    Ok(json!({"ok":true,"qualities":choices}))
}

fn validated_url(raw: &str) -> Result<Url, &'static str> {
    if raw.len() > MAX_URL_BYTES || raw.chars().any(|c| c.is_control()) {
        return Err("The URL is too long or contains control characters.");
    }
    let url = Url::parse(raw.trim()).map_err(|_| "Enter an absolute HTTP or HTTPS URL.")?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("Only HTTP and HTTPS download URLs are supported.");
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("URLs containing a username or password are not accepted.");
    }
    Ok(url)
}

fn read_message(input: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut header = [0u8; 4];
    loop {
        match input.read(&mut header[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => break,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    input.read_exact(&mut header[1..])?;
    let length = u32::from_ne_bytes(header) as usize;
    if length == 0 || length > MAX_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid native message length.",
        ));
    }
    let mut body = vec![0u8; length];
    input.read_exact(&mut body)?;
    Ok(Some(body))
}

fn write_message(output: &mut impl Write, value: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(value)?;
    output.write_all(&(body.len() as u32).to_ne_bytes())?;
    output.write_all(&body)?;
    output.flush()
}

fn idx_command(executable: &Path) -> Command {
    let mut command = Command::new(executable);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    command
}

fn launch_idx(
    executable: &Path,
    url: &Url,
    automatic: bool,
    parent_page: &str,
    filename: &str,
    referer: &str,
    browser: &browser_session::BrowserContext,
    stream: &streaming::StreamOptions,
) -> Result<(), String> {
    if !executable.is_file() {
        return Err("IDX is missing. Reinstall IDX or register the correct native host.".into());
    }
    let mut enqueue = idx_command(executable);
    enqueue.arg("--enqueue-json").stdin(Stdio::piped());
    let mut child = enqueue
        .spawn()
        .map_err(|_| "IDX could not accept the download.")?;
    let payload = json!({"url":url.as_str(),"automatic":automatic,"parent_page":parent_page,"filename":filename,"referer":referer,"browser":browser,"stream":stream});
    let write = child
        .stdin
        .take()
        .ok_or("Could not open private browser handoff pipe.")
        .and_then(|mut input| {
            input
                .write_all(&serde_json::to_vec(&payload).unwrap())
                .map_err(|_| "Could not write private browser handoff.")
        });
    if write.is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return Err("Could not pass the browser session to IDX.".into());
    }
    let status = child
        .wait()
        .map_err(|_| "IDX could not acknowledge the browser request.")?;
    if !status.success() {
        return Err(
            "IDX could not save the download request. The browser download has not been stopped."
                .into(),
        );
    }
    // The short-lived CLI exits successfully only after saving the inbox entry.
    // Start/focus the GUI separately; never wait for the normal application to exit.
    idx_command(executable)
        .spawn()
        .map(|_| ())
        .map_err(|_| "The request was saved, but IDX could not open. The browser download has not been stopped.".into())
}

fn serve(
    input: &mut impl Read,
    output: &mut impl Write,
    mut launch: impl FnMut(
        &Url,
        bool,
        &str,
        &str,
        &str,
        &browser_session::BrowserContext,
        &streaming::StreamOptions,
    ) -> Result<(), String>,
) -> io::Result<()> {
    loop {
        let body = match read_message(input) {
            Ok(Some(body)) => body,
            Ok(None) => return Ok(()),
            Err(error) => {
                // A malformed frame cannot be safely resynchronized. Send one error and stop.
                write_message(
                    output,
                    &json!({"ok": false, "error": "Invalid or truncated native message frame."}),
                )?;
                return Err(error);
            }
        };
        let request = match serde_json::from_slice::<Request>(&body) {
            Ok(Request::Ping(_)) => {
                write_message(
                    output,
                    &json!({"ok": true, "version": env!("CARGO_PKG_VERSION"), "protocol": 2}),
                )?;
                continue;
            }
            Ok(Request::Inspect(request)) => {
                let reply =
                    inspect(request).unwrap_or_else(|error| json!({"ok":false,"error":error}));
                write_message(output, &reply)?;
                continue;
            }
            Ok(Request::Download(request)) => Ok(request),
            Err(_) => Err("Expected a download request or a connection check.".to_string()),
        };
        let result = request
            .and_then(|request| {
                browser_media::validate_filename(&request.filename).map_err(str::to_owned)?;
                request.browser.validate()?;
                request.stream.validate()?;
                let referer = if request.referer.is_empty() {
                    String::new()
                } else {
                    validated_url(&request.referer)
                        .map_err(str::to_owned)?
                        .to_string()
                };
                let parent_page = if request.parent_page.is_empty() {
                    String::new()
                } else {
                    validated_url(&request.parent_page)
                        .map_err(str::to_owned)?
                        .to_string()
                };
                validated_url(&request.url)
                    .map(|url| {
                        (
                            url,
                            request.automatic,
                            parent_page,
                            request.filename,
                            referer,
                            request.browser,
                            request.stream,
                        )
                    })
                    .map_err(str::to_owned)
            })
            .and_then(
                |(url, automatic, parent_page, filename, referer, browser, stream)| {
                    launch(
                        &url,
                        automatic,
                        &parent_page,
                        &filename,
                        &referer,
                        &browser,
                        &stream,
                    )
                },
            );
        let response = match result {
            Ok(()) => json!({"ok": true, "queued": true, "message": "Download saved in IDX."}),
            Err(error) => json!({"ok": false, "error": error}),
        };
        write_message(output, &response)?;
    }
}

fn main() {
    let result = (|| -> io::Result<()> {
        let host = std::env::current_exe()?;
        let directory = host
            .parent()
            .ok_or_else(|| io::Error::other("Missing application directory."))?;
        let executable = directory.join(if cfg!(windows) { "idx.exe" } else { "idx" });
        if !executable.is_file() {
            return Err(io::Error::other(
                "IDX is missing. Reinstall the desktop app.",
            ));
        }
        serve(
            &mut io::stdin().lock(),
            &mut io::stdout().lock(),
            |url, automatic, parent_page, filename, referer, browser, stream| {
                launch_idx(
                    &executable,
                    url,
                    automatic,
                    parent_page,
                    filename,
                    referer,
                    browser,
                    stream,
                )
            },
        )
    })();
    if let Err(error) = result {
        eprintln!("IDX native host: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn frame(json: &str) -> Vec<u8> {
        let mut result = (json.len() as u32).to_ne_bytes().to_vec();
        result.extend_from_slice(json.as_bytes());
        result
    }

    fn responses(bytes: Vec<u8>) -> Vec<Value> {
        let mut input = Cursor::new(bytes);
        let mut result = Vec::new();
        while let Some(body) = read_message(&mut input).unwrap() {
            result.push(serde_json::from_slice(&body).unwrap());
        }
        result
    }

    #[test]
    fn connection_check_does_not_launch_the_app_or_queue_a_download() {
        let mut output = Vec::new();
        serve(
            &mut Cursor::new(frame(r#"{"action":"ping"}"#)),
            &mut output,
            |_, _, _, _, _, _, _| panic!("connection check must not launch"),
        )
        .unwrap();
        let replies = responses(output);
        assert_eq!(replies[0]["ok"], true);
        assert_eq!(replies[0]["protocol"], 2);
        assert_eq!(replies[0]["version"], env!("CARGO_PKG_VERSION"));
        assert!(replies[0].get("queued").is_none());
    }

    #[test]
    fn browser_media_metadata_is_validated_before_launch() {
        let mut output = Vec::new();
        serve(&mut Cursor::new(frame(r#"{"url":"https://files.example/play?id=1","automatic":true,"filename":"Film.mp4","referer":"https://site.example/watch","parent_page":"https://site.example/watch"}"#)), &mut output,
            |_, automatic, parent, filename, referer, _, _| {
                assert!(automatic);
                assert_eq!(filename,"Film.mp4");
                assert_eq!(parent,referer);
                Ok(())
            }).unwrap();
        assert_eq!(responses(output)[0]["queued"], true);
        for body in [
            r#"{"url":"https://files.example/v","filename":"../bad.exe"}"#,
            r#"{"url":"https://files.example/v","referer":"file:///secret"}"#,
        ] {
            let mut output = Vec::new();
            serve(
                &mut Cursor::new(frame(body)),
                &mut output,
                |_, _, _, _, _, _, _| panic!("invalid metadata must not launch"),
            )
            .unwrap();
            assert_eq!(responses(output)[0]["ok"], false);
        }
    }

    #[test]
    fn parent_page_is_validated_and_forwarded_separately() {
        let mut output = Vec::new();
        serve(&mut Cursor::new(frame(r#"{"url":"https://files.example/a.zip","parent_page":"https://site.example/page"}"#)), &mut output,
            |url, automatic, parent, _, _, _, _| {
                assert_eq!(url.as_str(), "https://files.example/a.zip");
                assert!(!automatic);
                assert_eq!(parent, "https://site.example/page");
                Ok(())
            }).unwrap();
        assert_eq!(responses(output)[0]["queued"], true);
        for body in [
            r#"{"url":"https://files.example/a.zip","parent_page":"file:///private"}"#,
            r#"{"action":"ping","url":"https://files.example/a.zip"}"#,
            r#"{"action":"run"}"#,
        ] {
            let mut output = Vec::new();
            serve(
                &mut Cursor::new(frame(body)),
                &mut output,
                |_, _, _, _, _, _, _| panic!("invalid request"),
            )
            .unwrap();
            assert_eq!(responses(output)[0]["ok"], false);
        }
    }

    #[test]
    fn validates_network_urls_without_credentials() {
        assert_eq!(
            validated_url(" https://example.com/a?q=1 ")
                .unwrap()
                .as_str(),
            "https://example.com/a?q=1"
        );
        for bad in [
            "file:///C:/secret",
            "javascript:alert(1)",
            "ftp://example.com/f",
            "https://user:pass@example.com/f",
            "https://user@example.com/f",
            "https://example.com/\nfile",
            "relative.zip",
        ] {
            assert!(validated_url(bad).is_err(), "accepted {bad}");
        }
        assert!(
            validated_url(&format!(
                "https://example.com/{}",
                "x".repeat(MAX_URL_BYTES)
            ))
            .is_err()
        );
    }

    #[test]
    fn handles_multiple_requests_and_recovers_from_invalid_json() {
        let mut input = frame(r#"{"url":"https://example.com/file.zip"}"#);
        input.extend(frame("not json"));
        input.extend(frame(r#"{"url":"http://localhost:8000/file"}"#));
        let mut output = Vec::new();
        let mut urls = Vec::new();
        serve(
            &mut Cursor::new(input),
            &mut output,
            |url, automatic, _, _, _, _, _| {
                assert!(!automatic);
                urls.push(url.to_string());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            urls,
            ["https://example.com/file.zip", "http://localhost:8000/file"]
        );
        let replies = responses(output);
        assert_eq!(replies.len(), 3);
        assert_eq!(replies[0]["ok"], true);
        assert_eq!(replies[1]["ok"], false);
        assert_eq!(replies[2]["ok"], true);
    }

    #[test]
    fn never_launches_bad_urls_or_unexpected_fields() {
        for request in [
            r#"{"url":"file:///C:/secret"}"#,
            r#"{"url":"https://user:secret@example.com"}"#,
            r#"{"url":"https://example.com","command":"something"}"#,
            r#"{"url":42}"#,
        ] {
            let mut output = Vec::new();
            serve(
                &mut Cursor::new(frame(request)),
                &mut output,
                |_, _, _, _, _, _, _| panic!("must not launch"),
            )
            .unwrap();
            let replies = responses(output);
            assert_eq!(replies[0]["ok"], false);
            assert!(!replies[0].to_string().contains("secret"));
        }
    }

    #[test]
    fn rejects_oversized_and_partial_frames() {
        for input in [
            vec![1, 2],
            (MAX_MESSAGE_BYTES as u32 + 1).to_ne_bytes().to_vec(),
            vec![5, 0, 0, 0, b'{'],
            vec![0, 0, 0, 0],
        ] {
            let mut output = Vec::new();
            assert!(
                serve(
                    &mut Cursor::new(input),
                    &mut output,
                    |_, _, _, _, _, _, _| panic!("must not launch")
                )
                .is_err()
            );
            assert_eq!(responses(output)[0]["ok"], false);
        }
    }

    #[test]
    fn reports_launch_errors_and_clean_eof() {
        let mut output = Vec::new();
        serve(
            &mut Cursor::new(frame(r#"{"url":"https://example.com"}"#)),
            &mut output,
            |_, _, _, _, _, _, _| Err("IDX could not start.".into()),
        )
        .unwrap();
        assert_eq!(responses(output)[0]["error"], "IDX could not start.");
        let mut empty = Vec::new();
        serve(
            &mut Cursor::new(Vec::<u8>::new()),
            &mut empty,
            |_, _, _, _, _, _, _| panic!("must not launch"),
        )
        .unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn automatic_requests_require_boolean_and_acknowledge_saved_queue() {
        let mut output = Vec::new();
        serve(
            &mut Cursor::new(frame(r#"{"url":"https://example.com/f","automatic":true}"#)),
            &mut output,
            |_, automatic, _, _, _, _, _| {
                assert!(automatic);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(responses(output)[0]["queued"], true);
        for request in [
            r#"{"url":"https://example.com","automatic":"yes"}"#,
            r#"{"url":"https://example.com","automatic":null}"#,
        ] {
            let mut output = Vec::new();
            serve(
                &mut Cursor::new(frame(request)),
                &mut output,
                |_, _, _, _, _, _, _| panic!("must not enqueue"),
            )
            .unwrap();
            assert_eq!(responses(output)[0]["ok"], false);
        }
    }
}
