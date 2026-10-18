//! Durable browser handoff. A successful CLI response means the inbox item is
//! flushed and published; the main application acknowledges it after saving.
use serde::{Deserialize, Serialize};
use std::{io::Write, path::Path};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub url: String,
    #[serde(default, skip_serializing)]
    pub browser: crate::browser_session::BrowserContext,
    #[serde(default)]
    pub protected_context: Vec<u8>,
    #[serde(default)]
    pub stream: crate::streaming::StreamOptions,
    #[serde(default)]
    pub automatic: bool,
    #[serde(default)]
    pub parent_page: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub referer: String,
}

pub fn enqueue(inbox: &Path, url: &str, automatic: bool) -> Result<(), String> {
    enqueue_with_parent(inbox, url, automatic, "")
}

pub fn enqueue_with_parent(
    inbox: &Path,
    url: &str,
    automatic: bool,
    parent_page: &str,
) -> Result<(), String> {
    enqueue_with_metadata(inbox, url, automatic, parent_page, "", "")
}

pub fn enqueue_with_metadata(
    inbox: &Path,
    url: &str,
    automatic: bool,
    parent_page: &str,
    filename: &str,
    referer: &str,
) -> Result<(), String> {
    enqueue_request(
        inbox,
        Request {
            url: url.into(),
            automatic,
            parent_page: parent_page.into(),
            filename: filename.into(),
            referer: referer.into(),
            browser: Default::default(),
            protected_context: vec![],
            stream: Default::default(),
        },
    )
}
pub fn enqueue_request(inbox: &Path, mut request: Request) -> Result<(), String> {
    let url = &request.url;
    let filename = &request.filename;
    let referer = &request.referer;
    let parent_page = &request.parent_page;
    crate::browser_media::validate_filename(filename)?;
    if url.len() > 16_384 || url.chars().any(char::is_control) {
        return Err("Invalid download address".into());
    }
    crate::model::filename_from_url(url)?;
    if !referer.is_empty() {
        if referer.len() > 16_384 || referer.chars().any(char::is_control) {
            return Err("Invalid browser referer".into());
        }
        crate::model::filename_from_url(referer)?;
    }
    if !parent_page.is_empty() {
        if parent_page.len() > 16_384 || parent_page.chars().any(char::is_control) {
            return Err("Invalid parent page".into());
        }
        crate::model::filename_from_url(parent_page)?;
    }
    request.browser.validate()?;
    request.stream.validate()?;
    if !request.protected_context.is_empty() {
        return Err("Unexpected protected browser envelope.".into());
    }
    if !request.browser.cookies.is_empty() || !request.browser.user_agent.is_empty() {
        request.protected_context = crate::browser_session::crypt(
            &serde_json::to_vec(&request.browser).map_err(|_| "Invalid browser session.")?,
            false,
        )?;
    }
    std::fs::create_dir_all(inbox).map_err(|e| e.to_string())?;
    let id = format!(
        "{}-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        std::process::id()
    );
    let temporary = inbox.join(format!("{id}.tmp"));
    let destination = inbox.join(format!("{id}.json"));
    let bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(&temporary, &destination).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(windows)]
    fn browser_credentials_are_protected_in_the_durable_inbox() {
        let dir = std::env::temp_dir().join(format!(
            "idx-protected-inbox-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let request:Request=serde_json::from_value(serde_json::json!({"url":"https://example.com/v.m3u8","automatic":true,"stream":{"kind":"hls","max_height":720},"browser":{"cookies":[{"name":"session","value":"never-plain-cookie","domain":"example.com","path":"/","secure":true,"host_only":true}]}})).unwrap();
        enqueue_request(&dir, request).unwrap();
        let path = std::fs::read_dir(&dir)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let bytes = std::fs::read(&path).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("never-plain-cookie"));
        let request: Request = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            request.browser_context().unwrap().cookies[0].value,
            "never-plain-cookie"
        );
        assert_eq!(request.stream.max_height, 720);
        std::fs::remove_dir_all(&dir).unwrap();
    }
    #[test]
    fn browser_handoff_is_complete_and_durable_before_acknowledgement() {
        let dir = std::env::temp_dir().join(format!(
            "idx-inbox-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        enqueue(&dir, "https://example.com/file.zip", true).unwrap();
        let files = std::fs::read_dir(&dir)
            .unwrap()
            .map(|f| f.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].extension().unwrap(), "json");
        let request: Request = serde_json::from_slice(&std::fs::read(&files[0]).unwrap()).unwrap();
        assert!(request.automatic);
        assert_eq!(request.url, "https://example.com/file.zip");
        assert!(enqueue(&dir, "file:///C:/private", true).is_err());
        assert!(enqueue(&dir, "https://user:password@example.com/file", true).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

impl Request {
    pub fn browser_context(&self) -> Result<crate::browser_session::BrowserContext, String> {
        if self.protected_context.is_empty() {
            return Ok(Default::default());
        }
        let bytes = crate::browser_session::crypt(&self.protected_context, true)?;
        let context: crate::browser_session::BrowserContext =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid protected browser session.")?;
        context.validate()?;
        Ok(context)
    }
}
