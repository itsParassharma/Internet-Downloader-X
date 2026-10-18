//! HTTP download engine. Sidecar files are private to IDX; destination files are
//! published atomically without ever replacing an existing destination.
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{
    ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE, ETAG, IF_RANGE,
    LAST_MODIFIED, RANGE,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{self, Sender},
};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct DownloadSpec {
    pub browser: crate::browser_session::BrowserContext,
    pub stream: crate::streaming::StreamOptions,
    pub id: u64,
    pub url: String,
    pub destination: PathBuf,
    pub connections: usize,
    pub retries: u32,
    pub speed_limit_kib: u64,
    pub username: String,
    pub password: String,
    pub referer: String,
    pub user_agent: String,
    pub proxy: String,
}

#[derive(Clone)]
pub struct DownloadControl {
    shared: Arc<Shared>,
}

impl DownloadControl {
    pub fn pause(&self) {
        self.shared.paused.store(true, Ordering::SeqCst);
    }
    pub fn set_speed_limit(&self, kib: u64) {
        self.shared.limit.store(kib, Ordering::Relaxed);
    }
}

#[derive(Clone, Debug)]
pub struct DownloadEvent {
    pub id: u64,
    pub kind: EventKind,
}

#[derive(Clone, Debug)]
pub enum EventKind {
    Started {
        total: Option<u64>,
        resumable: bool,
    },
    Progress {
        downloaded: u64,
        total: Option<u64>,
        speed: f64,
        parts: Vec<PartProgress>,
    },
    Completed {
        path: PathBuf,
    },
    Paused,
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct PartProgress {
    pub downloaded: u64,
    pub total: u64,
}

struct Shared {
    paused: AtomicBool,
    stopped: AtomicBool,
    limit: AtomicU64,
    rate_clock: Mutex<RateClock>,
}

struct RateClock {
    last: Instant,
    tokens: f64,
    limit: u64,
}

impl Shared {
    fn interrupted(&self) -> bool {
        self.paused.load(Ordering::Relaxed) || self.stopped.load(Ordering::Relaxed)
    }
    fn check(&self) -> Result<(), Failure> {
        if self.interrupted() {
            Err(Failure::Paused)
        } else {
            Ok(())
        }
    }
    fn delay(&self, duration: Duration) -> Result<(), Failure> {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            self.check()?;
            thread::sleep(
                deadline
                    .saturating_duration_since(Instant::now())
                    .min(Duration::from_millis(50)),
            );
        }
        self.check()
    }
    fn throttle(&self, bytes: usize) -> Result<(), Failure> {
        let mut remaining = bytes as f64;
        while remaining > 0.0 {
            self.check()?;
            let limit = self.limit.load(Ordering::Relaxed);
            if limit == 0 {
                return Ok(());
            }
            {
                let mut clock = self
                    .rate_clock
                    .lock()
                    .map_err(|_| Failure::Fatal("Download rate control failed.".into()))?;
                let now = Instant::now();
                let rate = limit as f64 * 1024.0;
                if clock.limit != limit {
                    clock.tokens = 0.0;
                    clock.limit = limit;
                }
                clock.tokens = (clock.tokens + now.duration_since(clock.last).as_secs_f64() * rate)
                    .min((rate * 0.1).max(1.0));
                clock.last = now;
                let consumed = clock.tokens.min(remaining);
                clock.tokens -= consumed;
                remaining -= consumed;
            }
            if remaining > 0.0 {
                self.delay(Duration::from_millis(20))?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
enum Failure {
    Paused,
    Retry(String),
    Fatal(String),
}

impl Failure {
    fn message(self) -> String {
        match self {
            Self::Paused => "Download paused.".into(),
            Self::Retry(s) | Self::Fatal(s) => s,
        }
    }
}

fn io_error(context: &str, error: std::io::Error) -> Failure {
    Failure::Fatal(format!("{context}: {error}"))
}
fn request_error(error: reqwest::Error) -> Failure {
    if error.is_timeout() {
        Failure::Retry("The server timed out.".into())
    } else if error.is_connect() {
        Failure::Retry(
            "Could not connect to the server. Check the connection, proxy, and certificate.".into(),
        )
    } else if error.is_builder() {
        Failure::Fatal("The URL, proxy, or request headers are invalid.".into())
    } else {
        Failure::Retry("The HTTP connection failed.".into())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Validator {
    header: String,
    value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Segment {
    start: u64,
    length: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Manifest {
    format: String,
    source_hash: String,
    total: Option<u64>,
    ranged: bool,
    validator: Option<Validator>,
    segments: Vec<Segment>,
}

struct Probe {
    total: Option<u64>,
    ranged: bool,
    validator: Option<Validator>,
}

pub fn spawn(spec: DownloadSpec, sender: Sender<DownloadEvent>) -> DownloadControl {
    let shared = Arc::new(Shared {
        paused: AtomicBool::new(false),
        stopped: AtomicBool::new(false),
        limit: AtomicU64::new(spec.speed_limit_kib),
        rate_clock: Mutex::new(RateClock {
            last: Instant::now(),
            tokens: 0.0,
            limit: spec.speed_limit_kib,
        }),
    });
    let control = DownloadControl {
        shared: shared.clone(),
    };
    thread::spawn(move || {
        let id = spec.id;
        let result = if spec.stream.kind.is_empty() {
            run(&spec, &sender, &shared)
        } else {
            stream_download::run(&spec, &sender, &shared)
        };
        let kind = match result {
            Ok(()) => EventKind::Completed {
                path: spec.destination.clone(),
            },
            Err(Failure::Paused) => EventKind::Paused,
            Err(error) => EventKind::Failed(error.message()),
        };
        let _ = sender.send(DownloadEvent { id, kind });
    });
    control
}

fn make_client(spec: &DownloadSpec) -> Result<Client, Failure> {
    let url = url::Url::parse(&spec.url)
        .map_err(|_| Failure::Fatal("Enter a valid HTTP or HTTPS URL.".into()))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(Failure::Fatal(
            "Only HTTP and HTTPS downloads are supported.".into(),
        ));
    }
    // Blocking ClientBuilder::timeout applies separately to each blocking read,
    // keeping stalled downloads cancellable without limiting total file time.
    let mut builder = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(15));
    if !spec.proxy.trim().is_empty() {
        builder = builder.proxy(
            reqwest::Proxy::all(spec.proxy.trim())
                .map_err(|_| Failure::Fatal("The proxy address is invalid.".into()))?,
        );
    }
    builder.build().map_err(request_error)
}

fn request(client: &Client, spec: &DownloadSpec) -> RequestBuilder {
    let mut request = client
        .get(&spec.url)
        .header(ACCEPT_ENCODING, "identity")
        .header(
            reqwest::header::USER_AGENT,
            if !spec.browser.user_agent.is_empty() {
                &spec.browser.user_agent
            } else if spec.user_agent.is_empty() {
                "IDX/0.1 InternetDownloaderX"
            } else {
                &spec.user_agent
            },
        );
    if !spec.username.is_empty() {
        request = request.basic_auth(&spec.username, Some(&spec.password));
    }
    if !spec.referer.is_empty() {
        request = request.header(reqwest::header::REFERER, &spec.referer);
    }
    request
}

fn validator(response: &Response) -> Option<Validator> {
    if let Some(value) = response.headers().get(ETAG).and_then(|v| v.to_str().ok()) {
        if !value.starts_with("W/") && value.starts_with('"') && value.ends_with('"') {
            return Some(Validator {
                header: "etag".into(),
                value: value.into(),
            });
        }
    }
    response
        .headers()
        .get(LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(|value| Validator {
            header: "last-modified".into(),
            value: value.into(),
        })
}

fn identity_encoding(response: &Response) -> Result<(), Failure> {
    if let Some(encoding) = response.headers().get(CONTENT_ENCODING) {
        if !encoding
            .to_str()
            .map(|s| s.eq_ignore_ascii_case("identity"))
            .unwrap_or(false)
        {
            return Err(Failure::Fatal(
                "The server ignored the identity encoding request; the file was not saved.".into(),
            ));
        }
    }
    Ok(())
}

fn status_error(status: reqwest::StatusCode) -> Failure {
    let message = match status.as_u16() {
        401 | 403 => "The server denied access. Check the link or website credentials.".to_string(),
        404 => "The file was not found on the server (HTTP 404).".to_string(),
        code => format!("The server returned HTTP {code}."),
    };
    if matches!(status.as_u16(), 408 | 425 | 429 | 500 | 502 | 503 | 504) {
        Failure::Retry(message)
    } else {
        Failure::Fatal(message)
    }
}

fn content_length(response: &Response) -> Result<Option<u64>, Failure> {
    response
        .headers()
        .get(CONTENT_LENGTH)
        .map(|h| {
            h.to_str()
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .ok_or_else(|| Failure::Fatal("The server sent an invalid content length.".into()))
        })
        .transpose()
}

fn parse_range(text: &str) -> Option<(u64, u64, u64)> {
    let (range, total) = text.strip_prefix("bytes ")?.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    let (start, end, total) = (
        start.parse::<u64>().ok()?,
        end.parse::<u64>().ok()?,
        total.parse::<u64>().ok()?,
    );
    (start <= end && end < total).then_some((start, end, total))
}

fn probe(client: &Client, spec: &DownloadSpec, shared: &Shared) -> Result<Probe, Failure> {
    let mut attempt = 0;
    loop {
        shared.check()?;
        let result = (|| {
            let response = crate::browser_session::send(
                client,
                request(client, spec).header(RANGE, "bytes=0-0"),
                &spec.browser,
            )
            .map_err(Failure::Retry)?;
            identity_encoding(&response)?;
            let identity = validator(&response);
            match response.status().as_u16() {
                206 => {
                    let range = response
                        .headers()
                        .get(CONTENT_RANGE)
                        .and_then(|h| h.to_str().ok())
                        .and_then(parse_range)
                        .ok_or_else(|| {
                            Failure::Fatal(
                                "The server sent an invalid Content-Range header.".into(),
                            )
                        })?;
                    if range.0 != 0
                        || range.1 != 0
                        || content_length(&response)?.is_some_and(|n| n != 1)
                    {
                        return Err(Failure::Fatal(
                            "The server returned the wrong byte range.".into(),
                        ));
                    }
                    Ok(Probe {
                        total: Some(range.2),
                        ranged: true,
                        validator: identity,
                    })
                }
                200 => Ok(Probe {
                    total: content_length(&response)?,
                    ranged: false,
                    validator: identity,
                }),
                416 if response
                    .headers()
                    .get(CONTENT_RANGE)
                    .and_then(|h| h.to_str().ok())
                    == Some("bytes */0") =>
                {
                    Ok(Probe {
                        total: Some(0),
                        ranged: false,
                        validator: identity,
                    })
                }
                _ => Err(status_error(response.status())),
            }
        })();
        match result {
            Err(Failure::Retry(_)) if attempt < spec.retries => {
                attempt += 1;
                shared.delay(Duration::from_millis(
                    500 * 2u64.saturating_pow(attempt.min(5)),
                ))?;
            }
            result => return result,
        }
    }
}

fn sidecar_path(destination: &Path) -> Result<PathBuf, Failure> {
    let mut filename = destination
        .file_name()
        .ok_or_else(|| Failure::Fatal("The destination needs a filename.".into()))?
        .to_os_string();
    filename.push(".idx.parts");
    Ok(destination.with_file_name(filename))
}

fn acquire_lock(directory: &Path) -> Result<File, Failure> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    let file = options.open(directory.join("active.lock")).map_err(|_| {
        Failure::Fatal(
            "This destination is already being downloaded, or its resume folder is locked.".into(),
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        unsafe extern "C" {
            fn flock(fd: i32, operation: i32) -> i32;
        }
        if unsafe { flock(file.as_raw_fd(), 2 | 4) } != 0 {
            return Err(Failure::Fatal(
                "This destination is already being downloaded.".into(),
            ));
        }
    }
    Ok(file)
}

fn safe_regular(path: &Path) -> Result<(), Failure> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => Err(Failure::Fatal(
            "The resume folder contains an unexpected file type.".into(),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("Could not inspect the resume file", error)),
    }
}

fn publish_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn MoveFileW(existing: *const u16, new_name: *const u16) -> i32;
        }
        // Canonical Windows paths carry the extended-length prefix, including
        // for UNC shares. The destination itself must remain absent, so only
        // canonicalize its existing parent and append the final filename.
        let source = fs::canonicalize(source)?;
        let parent = destination.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "The destination has no parent directory.",
            )
        })?;
        let filename = destination.file_name().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "The destination has no filename.",
            )
        })?;
        let destination = fs::canonicalize(parent)?.join(filename);
        let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let destination: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        if source[..source.len() - 1].contains(&0)
            || destination[..destination.len() - 1].contains(&0)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "A filename contains a null character.",
            ));
        }
        // MoveFileW, unlike MoveFileExW(REPLACE_EXISTING), cannot replace a file.
        // Both paths share a parent volume, so this is an atomic rename.
        if unsafe { MoveFileW(source.as_ptr(), destination.as_ptr()) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        fs::hard_link(source, destination)?;
        fs::remove_file(source)
    }
}

fn prepare(spec: &DownloadSpec, probe: Probe) -> Result<(PathBuf, File, Manifest), Failure> {
    if spec
        .destination
        .try_exists()
        .map_err(|e| io_error("Could not inspect the destination", e))?
    {
        return Err(Failure::Fatal(
            "The destination already exists. Choose a different filename.".into(),
        ));
    }
    let parent = spec
        .destination
        .parent()
        .ok_or_else(|| Failure::Fatal("The destination folder is invalid.".into()))?;
    fs::create_dir_all(parent)
        .map_err(|e| io_error("Could not create the destination folder", e))?;
    let directory = sidecar_path(&spec.destination)?;
    let source_hash = format!("{:x}", Sha256::digest(spec.url.as_bytes()));
    let new_directory = match fs::create_dir(&directory) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if !fs::symlink_metadata(&directory)
                .map_err(|e| io_error("Could not inspect the resume folder", e))?
                .file_type()
                .is_dir()
            {
                return Err(Failure::Fatal(
                    "The resume path is not a regular folder.".into(),
                ));
            }
            false
        }
        Err(error) => return Err(io_error("Could not create the resume folder", error)),
    };
    let manifest_path = directory.join("manifest.json");
    let existing = if new_directory {
        None
    } else {
        safe_regular(&manifest_path)?;
        let bytes = fs::read(&manifest_path).map_err(|_| {
            Failure::Fatal(
                "The existing resume folder is not owned by IDX. Choose another filename.".into(),
            )
        })?;
        let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|_| {
            Failure::Fatal(
                "The existing resume manifest is invalid. Choose another filename.".into(),
            )
        })?;
        if manifest.format != "IDX download v1" || manifest.source_hash != source_hash {
            return Err(Failure::Fatal(
                "This resume folder belongs to another download. Choose another filename.".into(),
            ));
        }
        Some(manifest)
    };
    safe_regular(&directory.join("active.lock"))?;
    let lock = acquire_lock(&directory)?;
    let count = if probe.ranged {
        spec.connections
            .clamp(1, 16)
            .min(probe.total.unwrap_or(1).clamp(1, 16) as usize)
    } else {
        1
    };
    let segments = if probe.ranged {
        let total = probe.total.unwrap_or(0);
        (0..count)
            .map(|index| {
                let start = (total as u128 * index as u128 / count as u128) as u64;
                let end = (total as u128 * (index + 1) as u128 / count as u128) as u64;
                Segment {
                    start,
                    length: Some(end - start),
                }
            })
            .collect()
    } else {
        vec![Segment {
            start: 0,
            length: probe.total,
        }]
    };
    let mut manifest = Manifest {
        format: "IDX download v1".into(),
        source_hash,
        total: probe.total,
        ranged: probe.ranged,
        validator: probe.validator,
        segments,
    };
    let reuse = existing.as_ref().is_some_and(|old| {
        old.ranged
            && manifest.ranged
            && old.total == manifest.total
            && old.validator.is_some()
            && old.validator == manifest.validator
            && valid_segments(old)
    });
    if reuse {
        manifest.segments = existing.as_ref().unwrap().segments.clone();
    } else if let Some(old) = &existing {
        // Only named IDX part files are ever removed. Unknown files are left alone.
        for index in 0..old.segments.len().min(16) {
            let path = directory.join(format!("{index}.part"));
            safe_regular(&path)?;
            if path.exists() {
                fs::remove_file(path)
                    .map_err(|e| io_error("Could not reset an outdated partial download", e))?;
            }
        }
    }
    let bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|_| Failure::Fatal("Could not save the resume manifest.".into()))?;
    safe_regular(&manifest_path)?;
    let temporary_manifest = directory.join("manifest.tmp");
    safe_regular(&temporary_manifest)?;
    let mut file = File::create(&temporary_manifest)
        .map_err(|e| io_error("Could not write the resume manifest", e))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| io_error("Could not save the resume manifest", e))?;
    drop(file);
    fs::rename(temporary_manifest, &manifest_path)
        .map_err(|e| io_error("Could not commit the resume manifest", e))?;
    Ok((directory, lock, manifest))
}

fn valid_segments(manifest: &Manifest) -> bool {
    if manifest.segments.is_empty() || manifest.segments.len() > 16 {
        return false;
    }
    let mut offset = 0u64;
    for segment in &manifest.segments {
        if segment.start != offset {
            return false;
        }
        let Some(length) = segment.length else {
            return false;
        };
        let Some(next) = offset.checked_add(length) else {
            return false;
        };
        offset = next;
    }
    Some(offset) == manifest.total
}

fn verify_range(
    response: &Response,
    start: u64,
    end: u64,
    total: u64,
    expected: &Option<Validator>,
) -> Result<(), Failure> {
    if response.status().as_u16() != 206 {
        if response.status().as_u16() == 200 {
            return Err(Failure::Fatal("The server changed the file or stopped supporting ranges. Resume again to restart safely.".into()));
        }
        return Err(status_error(response.status()));
    }
    identity_encoding(response)?;
    let actual = response
        .headers()
        .get(CONTENT_RANGE)
        .and_then(|h| h.to_str().ok())
        .and_then(parse_range);
    if actual != Some((start, end, total))
        || content_length(response)?.is_some_and(|n| n != end - start + 1)
    {
        return Err(Failure::Fatal(
            "The server returned an incorrect byte range. The destination file was not created."
                .into(),
        ));
    }
    if expected.is_some() && validator(response) != *expected {
        return Err(Failure::Fatal(
            "The remote file changed during download. Resume again to restart safely.".into(),
        ));
    }
    Ok(())
}

fn transfer_part(
    client: &Client,
    spec: &DownloadSpec,
    manifest: &Manifest,
    index: usize,
    path: &Path,
    progress: &AtomicU64,
    shared: &Shared,
) -> Result<(), Failure> {
    let segment = &manifest.segments[index];
    let mut failures = 0;
    loop {
        shared.check()?;
        let result = (|| {
            let mut existing = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            if !manifest.ranged || segment.length.is_some_and(|length| existing > length) {
                File::create(path).map_err(|e| io_error("Could not reset the partial file", e))?;
                existing = 0;
                progress.store(0, Ordering::Relaxed);
            }
            if segment.length == Some(existing) {
                return Ok(true);
            }
            let mut request = request(client, spec);
            let requested = if manifest.ranged {
                let start = segment.start + existing;
                // Bound each request, allowing retries without discarding completed bytes.
                let end = (start.saturating_add(8 * 1024 * 1024 - 1))
                    .min(segment.start + segment.length.unwrap() - 1);
                request = request.header(RANGE, format!("bytes={start}-{end}"));
                if let Some(identity) = &manifest.validator {
                    request = request.header(IF_RANGE, &identity.value);
                }
                Some((start, end))
            } else {
                None
            };
            let mut response = crate::browser_session::send(client, request, &spec.browser)
                .map_err(Failure::Retry)?;
            if let Some((start, end)) = requested {
                verify_range(
                    &response,
                    start,
                    end,
                    manifest.total.unwrap(),
                    &manifest.validator,
                )?;
            } else {
                if response.status().as_u16() != 200 {
                    return Err(status_error(response.status()));
                }
                identity_encoding(&response)?;
                if let (Some(expected), Some(actual)) = (manifest.total, content_length(&response)?)
                {
                    if expected != actual {
                        return Err(Failure::Fatal(
                            "The remote file size changed. Resume to restart safely.".into(),
                        ));
                    }
                }
                if manifest.validator.is_some() && validator(&response) != manifest.validator {
                    return Err(Failure::Fatal(
                        "The remote file changed. Resume to restart safely.".into(),
                    ));
                }
            }
            let expected = requested
                .map(|(start, end)| end - start + 1)
                .or(segment.length);
            let mut received = 0u64;
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| io_error("Could not write the partial file", e))?;
            let mut buffer = [0u8; 64 * 1024];
            loop {
                shared.check()?;
                let count = response
                    .read(&mut buffer)
                    .map_err(|_| Failure::Retry("The server interrupted the download.".into()))?;
                if count == 0 {
                    break;
                }
                if expected.is_some_and(|length| received + count as u64 > length) {
                    return Err(Failure::Fatal(
                        "The server returned too much data for the requested range.".into(),
                    ));
                }
                shared.throttle(count)?;
                file.write_all(&buffer[..count])
                    .map_err(|e| io_error("Could not save downloaded data", e))?;
                received += count as u64;
                progress.store(existing + received, Ordering::Relaxed);
            }
            file.sync_data()
                .map_err(|e| io_error("Could not flush downloaded data", e))?;
            if expected.is_some_and(|length| received != length) {
                return Err(Failure::Retry(
                    "The server ended the download before all bytes arrived.".into(),
                ));
            }
            Ok(!manifest.ranged || segment.length == Some(existing + received))
        })();
        match result {
            Ok(true) => return Ok(()),
            Ok(false) => {
                failures = 0;
            }
            Err(Failure::Retry(_)) if failures < spec.retries => {
                failures += 1;
                shared.delay(Duration::from_millis(
                    500 * 2u64.saturating_pow(failures.min(5)),
                ))?;
            }
            Err(error) => return Err(error),
        }
    }
}

fn run(
    spec: &DownloadSpec,
    sender: &Sender<DownloadEvent>,
    shared: &Arc<Shared>,
) -> Result<(), Failure> {
    let client = make_client(spec)?;
    let metadata = probe(&client, spec, shared)?;
    let (directory, lock, manifest) = prepare(spec, metadata)?;
    let send = |kind| {
        let _ = sender.send(DownloadEvent { id: spec.id, kind });
    };
    send(EventKind::Started {
        total: manifest.total,
        resumable: manifest.ranged && manifest.validator.is_some(),
    });
    let mut initial = Vec::new();
    for (index, segment) in manifest.segments.iter().enumerate() {
        let path = directory.join(format!("{index}.part"));
        safe_regular(&path)?;
        let length = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        initial.push(AtomicU64::new(
            if manifest.ranged && !segment.length.is_some_and(|total| length > total) {
                length
            } else {
                0
            },
        ));
    }
    let counts = Arc::new(initial);
    let (done_tx, done_rx) = mpsc::channel();
    let mut workers = Vec::new();
    for index in 0..manifest.segments.len() {
        let (client, spec, manifest, counts, shared, done_tx) = (
            client.clone(),
            spec.clone(),
            manifest.clone(),
            counts.clone(),
            shared.clone(),
            done_tx.clone(),
        );
        let path = directory.join(format!("{index}.part"));
        workers.push(thread::spawn(move || {
            let result = transfer_part(
                &client,
                &spec,
                &manifest,
                index,
                &path,
                &counts[index],
                &shared,
            );
            let _ = done_tx.send(result);
        }));
    }
    drop(done_tx);
    let mut remaining = workers.len();
    let mut failure = None;
    let mut previous = counts
        .iter()
        .map(|c| c.load(Ordering::Relaxed))
        .sum::<u64>();
    let mut tick = Instant::now();
    let mut speed = 0.0;
    while remaining > 0 {
        match done_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(result) => {
                remaining -= 1;
                if let Err(error) = result {
                    if !matches!(error, Failure::Paused) || failure.is_none() {
                        failure = Some(error);
                    }
                    shared.stopped.store(true, Ordering::Relaxed);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                failure = Some(Failure::Fatal(
                    "A download worker stopped unexpectedly.".into(),
                ));
                break;
            }
        }
        let parts: Vec<_> = counts
            .iter()
            .zip(&manifest.segments)
            .map(|(count, segment)| PartProgress {
                downloaded: count.load(Ordering::Relaxed),
                total: segment.length.unwrap_or(0),
            })
            .collect();
        let downloaded = parts.iter().map(|p| p.downloaded).sum::<u64>();
        let elapsed = tick.elapsed().as_secs_f64();
        if elapsed >= 0.15 {
            speed = 0.25 * downloaded.saturating_sub(previous) as f64 / elapsed + 0.75 * speed;
            previous = downloaded;
            tick = Instant::now();
        }
        send(EventKind::Progress {
            downloaded,
            total: manifest.total,
            speed,
            parts,
        });
    }
    for worker in workers {
        if worker.join().is_err() {
            failure = Some(Failure::Fatal(
                "A download worker failed unexpectedly.".into(),
            ));
        }
    }
    if let Some(error) = failure {
        return Err(error);
    }
    shared.check()?;
    let assembly_path = directory.join("assembled.tmp");
    safe_regular(&assembly_path)?;
    let mut assembled = File::create(&assembly_path)
        .map_err(|e| io_error("Could not assemble the downloaded file", e))?;
    let mut written = 0u64;
    let mut assembly_buffer = vec![0u8; 1024 * 1024];
    for (index, segment) in manifest.segments.iter().enumerate() {
        shared.check()?;
        let path = directory.join(format!("{index}.part"));
        // Empty downloads do not need a network body or an intermediate part.
        if segment.length == Some(0) && !path.exists() {
            continue;
        }
        let mut part =
            File::open(path).map_err(|e| io_error("Could not read a downloaded segment", e))?;
        let length = part
            .metadata()
            .map_err(|e| io_error("Could not inspect a downloaded segment", e))?
            .len();
        if segment.length.is_some_and(|expected| length != expected) {
            return Err(Failure::Fatal(
                "A downloaded segment has the wrong length.".into(),
            ));
        }
        loop {
            shared.check()?;
            let count = part
                .read(&mut assembly_buffer)
                .map_err(|error| io_error("Could not read a downloaded segment", error))?;
            if count == 0 {
                break;
            }
            assembled
                .write_all(&assembly_buffer[..count])
                .map_err(|error| io_error("Could not assemble the file", error))?;
            written += count as u64;
        }
    }
    if manifest.total.is_some_and(|total| written != total) {
        return Err(Failure::Fatal(
            "The assembled file has the wrong length.".into(),
        ));
    }
    assembled
        .sync_all()
        .map_err(|e| io_error("Could not flush the completed file", e))?;
    drop(assembled);
    shared.check()?;
    publish_file(&assembly_path, &spec.destination).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            Failure::Fatal("A file appeared at the destination. IDX did not replace it.".into())
        } else {
            io_error("Could not publish the completed file", error)
        }
    })?;
    drop(lock);
    // Never recursively delete: a user may have put unrelated files in the folder.
    for index in 0..manifest.segments.len() {
        let _ = fs::remove_file(directory.join(format!("{index}.part")));
    }
    let _ = fs::remove_file(assembly_path);
    let _ = fs::remove_file(directory.join("manifest.json"));
    let _ = fs::remove_file(directory.join("active.lock"));
    let _ = fs::remove_dir(directory);
    Ok(())
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;

#[path = "stream_download.rs"]
mod stream_download;
