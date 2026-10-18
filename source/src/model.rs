use chrono::{DateTime, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Status {
    #[default]
    Queued,
    Connecting,
    Downloading,
    Verifying,
    Paused,
    Complete,
    Error,
}
impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Connecting => "Connecting…",
            Self::Downloading => "Downloading",
            Self::Verifying => "Verifying…",
            Self::Paused => "Paused",
            Self::Complete => "Complete",
            Self::Error => "Error",
        }
    }
    pub fn active(self) -> bool {
        matches!(self, Self::Connecting | Self::Downloading)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Download {
    pub external_id: String,
    pub stream: crate::streaming::StreamOptions,
    #[serde(skip)]
    pub browser: crate::browser_session::BrowserContext,
    pub id: u64,
    pub url: String,
    pub path: PathBuf,
    pub category: String,
    pub queue: String,
    pub status: Status,
    pub total: Option<u64>,
    pub downloaded: u64,
    pub resumable: bool,
    pub added: DateTime<Utc>,
    pub last_try: Option<DateTime<Utc>>,
    pub description: String,
    pub error: String,
    pub sha256: String,
    pub verified_sha256: String,
    pub referer: String,
    /// Page from which the link was discovered, independent of HTTP Referer.
    pub parent_page: String,
    /// None follows the application limit; Some(0) explicitly means unlimited.
    pub speed_limit_kib: Option<u64>,
    pub notify_on_complete: bool,
    pub open_folder_on_complete: bool,
    pub close_progress_on_complete: bool,
    #[serde(skip)]
    pub speed: f64,
    #[serde(skip)]
    pub parts: Vec<(u64, u64)>,
    #[serde(skip)]
    pub username: String,
    #[serde(skip)]
    pub password: String,
    #[serde(skip)]
    pub log: Vec<String>,
}
impl Default for Download {
    fn default() -> Self {
        Self {
            external_id: String::new(),
            stream: Default::default(),
            browser: Default::default(),
            id: 0,
            url: String::new(),
            path: PathBuf::new(),
            category: "Other".into(),
            queue: "Main download queue".into(),
            status: Status::Queued,
            total: None,
            downloaded: 0,
            resumable: false,
            added: Utc::now(),
            last_try: None,
            description: String::new(),
            error: String::new(),
            sha256: String::new(),
            verified_sha256: String::new(),
            referer: String::new(),
            parent_page: String::new(),
            speed_limit_kib: None,
            notify_on_complete: true,
            open_folder_on_complete: false,
            close_progress_on_complete: false,
            speed: 0.0,
            parts: vec![],
            username: String::new(),
            password: String::new(),
            log: vec![],
        }
    }
}
impl Download {
    pub fn finish_verification(&mut self, result: Result<String, String>) {
        if self.status != Status::Verifying {
            return;
        }
        match result {
            Ok(actual) if actual.eq_ignore_ascii_case(&self.sha256) => {
                self.verified_sha256 = actual.clone();
                self.status = Status::Complete;
                self.error.clear();
                self.note(format!("SHA-256 verified: {actual}"));
            }
            result => {
                let error = result
                    .err()
                    .unwrap_or_else(|| "The expected checksum changed during verification.".into());
                self.verified_sha256.clear();
                self.status = Status::Error;
                self.error = error.clone();
                self.note(error);
            }
        }
    }
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
    pub fn percent(&self) -> f32 {
        self.total
            .filter(|&t| t > 0)
            .map(|t| (self.downloaded as f64 / t as f64).clamp(0.0, 1.0) as f32)
            .unwrap_or(0.0)
    }
    pub fn note(&mut self, s: impl Into<String>) {
        self.log
            .push(format!("{}  {}", Local::now().format("%H:%M:%S"), s.into()));
        if self.log.len() > 150 {
            self.log.remove(0);
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Queue {
    pub name: String,
    pub parallel: usize,
    pub active: bool,
    pub scheduled: bool,
    pub start_hour: u32,
    pub start_minute: u32,
    pub stop_hour: u32,
    pub stop_minute: u32,
    pub stop_enabled: bool,
    pub weekdays: [bool; 7],
    /// None retains the weekly schedule used by existing libraries.
    pub schedule_date: Option<NaiveDate>,
    pub start_on_startup: bool,
    /// Local minute of the last automatic transition, persisted against replay.
    pub last_schedule_event: Option<i64>,
}
impl Default for Queue {
    fn default() -> Self {
        Self {
            name: "Main download queue".into(),
            parallel: 2,
            active: false,
            scheduled: false,
            start_hour: 0,
            start_minute: 0,
            stop_hour: 7,
            stop_minute: 0,
            stop_enabled: false,
            weekdays: [true; 7],
            schedule_date: None,
            start_on_startup: false,
            last_schedule_event: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub download_dir: PathBuf,
    pub category_folders: bool,
    pub categories: Vec<crate::categories::Category>,
    pub remember_last_path: bool,
    pub connections: usize,
    pub retries: u32,
    pub speed_limit_kib: u64,
    pub proxy: String,
    pub user_agent: String,
    pub dark: bool,
    pub queues: Vec<Queue>,
    pub confirm_delete: bool,
    pub columns: crate::columns::ColumnPreferences,
    pub grabber_filters: Vec<crate::grabber::filters::NamedFilter>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            download_dir: dirs::download_dir()
                .unwrap_or_else(|| PathBuf::from("Downloads"))
                .join("IDX"),
            category_folders: false,
            categories: crate::categories::defaults(),
            remember_last_path: false,
            connections: 8,
            retries: 3,
            speed_limit_kib: 0,
            proxy: String::new(),
            user_agent: concat!("IDX/", env!("CARGO_PKG_VERSION")).into(),
            dark: false,
            queues: vec![Queue::default()],
            confirm_delete: true,
            columns: Vec::new(),
            grabber_filters: crate::grabber::filters::default_filters(),
        }
    }
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub version: u32,
    pub next_id: u64,
    pub downloads: Vec<Download>,
    pub settings: Settings,
    #[serde(skip)]
    pub recovery_notice: Option<String>,
}
impl Library {
    pub fn rename_queue(&mut self, index: usize, name: &str) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 128 || name.chars().any(char::is_control) {
            return Err(
                "Enter a queue name of 1 to 128 characters without control characters.".into(),
            );
        }
        if self
            .settings
            .queues
            .iter()
            .enumerate()
            .any(|(i, queue)| i != index && queue.name.to_lowercase() == name.to_lowercase())
        {
            return Err("A queue with that name already exists.".into());
        }
        let queue = self
            .settings
            .queues
            .get_mut(index)
            .ok_or("The queue no longer exists.")?;
        let old = std::mem::replace(&mut queue.name, name.to_string());
        for download in &mut self.downloads {
            if download.queue == old {
                download.queue = name.to_string();
            }
        }
        Ok(old)
    }

    pub fn prepare_added(&mut self, ids: &[u64], queue: &str, start: bool) {
        for download in &mut self.downloads {
            if ids.contains(&download.id) {
                download.status = if start {
                    Status::Queued
                } else {
                    Status::Paused
                };
            }
        }
        if start {
            if let Some(queue) = self
                .settings
                .queues
                .iter_mut()
                .find(|item| item.name == queue)
            {
                queue.active = true;
            }
        }
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        let backup_path = path.with_extension("json.bak");
        let mut recovered = false;
        let mut lib: Self = if !path.exists() {
            if let Ok(bytes) = std::fs::read(&backup_path) {
                if let Ok(mut backup) = serde_json::from_slice::<Self>(&bytes) {
                    backup.recovery_notice =
                        Some("The missing download library was restored from its backup.".into());
                    recovered = true;
                    backup
                } else {
                    Self {
                        version: 1,
                        next_id: 1,
                        ..Self::default()
                    }
                }
            } else {
                Self {
                    version: 1,
                    next_id: 1,
                    ..Self::default()
                }
            }
        } else {
            let bytes = std::fs::read(path).map_err(|e| format!("The library could not be read: {e}. Automatic saving is disabled to preserve it."))?;
            match serde_json::from_slice::<Self>(&bytes) {
                Ok(lib) => lib,
                Err(_) => {
                    let stamp = Utc::now().timestamp_nanos_opt().unwrap_or_default();
                    let mut preserved = path.with_extension(format!("corrupt-{stamp}.json"));
                    let mut suffix = 1;
                    while preserved.exists() {
                        preserved = path.with_extension(format!("corrupt-{stamp}-{suffix}.json"));
                        suffix += 1;
                    }
                    std::fs::rename(path, &preserved).map_err(|e| format!("The damaged library could not be preserved: {e}. Automatic saving is disabled."))?;
                    let backup = std::fs::read(&backup_path)
                        .ok()
                        .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok());
                    let restored = backup.is_some();
                    let mut lib = backup.unwrap_or_else(|| Self {
                        version: 1,
                        next_id: 1,
                        ..Self::default()
                    });
                    lib.recovery_notice = Some(format!(
                        "The damaged library was preserved at {}. {}",
                        preserved.display(),
                        if restored {
                            "Your backup was restored."
                        } else {
                            "A new library was created; downloaded files were not changed."
                        }
                    ));
                    recovered = true;
                    lib
                }
            }
        };
        lib.next_id = lib.next_id.max(
            lib.downloads
                .iter()
                .map(|d| d.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        );
        for d in &mut lib.downloads {
            if d.status.active() {
                d.status = Status::Paused;
            }
            if d.status == Status::Complete
                && !d.sha256.is_empty()
                && !d.verified_sha256.eq_ignore_ascii_case(&d.sha256)
            {
                d.status = Status::Verifying;
            }
            if d.status == Status::Verifying && d.sha256.is_empty() {
                d.status = Status::Error;
                d.error = "Checksum verification could not resume because the expected checksum is missing.".into();
            }
        }
        for q in &mut lib.settings.queues {
            q.active = q.start_on_startup;
            if q.start_on_startup {
                for download in &mut lib.downloads {
                    if download.queue == q.name && download.status == Status::Paused {
                        download.status = Status::Queued;
                    }
                }
            }
        }
        if lib.settings.queues.is_empty() {
            lib.settings.queues.push(Queue::default());
        }
        if recovered {
            lib.save(path)?;
        }
        Ok(lib)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension("json.tmp");
        let mut sanitized = serde_json::to_value(self).map_err(|e| e.to_string())?;
        if let Some(proxy) = sanitized.pointer_mut("/settings/proxy") {
            if proxy.as_str().is_some_and(|s| {
                url::Url::parse(s).is_ok_and(|u| u.password().is_some() || !u.username().is_empty())
            }) {
                *proxy = serde_json::Value::String(String::new());
            }
        }
        let data = serde_json::to_vec_pretty(&sanitized).map_err(|e| e.to_string())?;
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        file.write_all(&data)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        if path.exists() {
            std::fs::copy(path, path.with_extension("json.bak")).map_err(|e| e.to_string())?;
        }
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())
    }
}
pub const CATEGORIES: &[&str] = &[
    "Compressed",
    "Documents",
    "Music",
    "Programs",
    "Video",
    "Other",
];
pub fn safe_filename(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .take(180)
        .collect();
    s = s.trim().trim_end_matches(['.', ' ']).to_owned();
    let stem = s.split('.').next().unwrap_or("").to_uppercase();
    if s.is_empty() {
        s = "download.bin".into();
    }
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.ends_with(['1', '2', '3', '4', '5', '6', '7', '8', '9']))
    {
        s = format!("_{s}");
    }
    s
}
pub fn filename_from_url(input: &str) -> Result<String, String> {
    let u = url::Url::parse(input.trim()).map_err(|e| format!("Invalid URL: {e}"))?;
    if !matches!(u.scheme(), "http" | "https") || u.host_str().is_none() {
        return Err("Enter an HTTP or HTTPS download URL.".into());
    }
    if !u.username().is_empty() || u.password().is_some() {
        return Err("Use the separate username and password fields instead of embedding credentials in the URL.".into());
    }
    let raw = u
        .path_segments()
        .and_then(|mut s| s.next_back())
        .filter(|s| !s.is_empty())
        .unwrap_or("download.bin");
    Ok(safe_filename(&decode_percent(raw)))
}
fn decode_percent(s: &str) -> String {
    let mut bytes = Vec::new();
    let data = s.as_bytes();
    let mut i = 0;
    while i < data.len() {
        if data[i] == b'%' && i + 2 < data.len() {
            if let Ok(hex) = std::str::from_utf8(&data[i + 1..i + 3]) {
                if let Ok(n) = u8::from_str_radix(hex, 16) {
                    bytes.push(n);
                    i += 3;
                    continue;
                }
            }
        }
        bytes.push(data[i]);
        i += 1;
    }
    String::from_utf8_lossy(&bytes).into_owned()
}
pub fn unique_path(base: &Path, name: &str, downloads: &[Download]) -> PathBuf {
    let mut path = base.join(name);
    let stem = Path::new(name)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let ext = Path::new(name)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let mut n = 1;
    while path.exists()
        || partial_path(&path).exists()
        || downloads.iter().any(|d| {
            d.path
                .to_string_lossy()
                .eq_ignore_ascii_case(&path.to_string_lossy())
        })
    {
        path = base.join(format!("{stem} ({n}){ext}"));
        n += 1;
    }
    path
}
pub fn partial_path(destination: &Path) -> PathBuf {
    let mut name = destination.file_name().unwrap_or_default().to_os_string();
    name.push(".idx.parts");
    destination.with_file_name(name)
}
pub fn verify_sha256(path: &Path, expected: &str) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)
        .map_err(|error| format!("Could not open the file for checksum verification: {error}"))?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65_536];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("Checksum verification could not read the file: {error}"))?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    let actual = format!("{:x}", hash.finalize());
    if actual.eq_ignore_ascii_case(expected) {
        Ok(actual)
    } else {
        Err(format!("SHA-256 mismatch. Actual: {actual}"))
    }
}
pub fn human_bytes(bytes: u64) -> String {
    let mut b = bytes as f64;
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut i = 0;
    while b >= 1024.0 && i < 4 {
        b /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{b:.2} {}", units[i])
    }
}
pub fn human_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "—".into();
    }
    let s = seconds as u64;
    if s >= 3600 {
        format!("{}h {:02}m", s / 3600, s % 3600 / 60)
    } else if s >= 60 {
        format!("{}m {:02}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_TEST: AtomicU64 = AtomicU64::new(1);
    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "idx-model-test-{}-{}",
                std::process::id(),
                NEXT_TEST.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn library(&self) -> PathBuf {
            self.0.join("library.json")
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn names_are_safe() {
        assert_eq!(safe_filename("../../bad?.exe"), ".._.._bad_.exe");
        assert_eq!(safe_filename("NUL.txt"), "_NUL.txt");
        assert_eq!(
            filename_from_url("https://example.com/a%20b.zip").unwrap(),
            "a b.zip"
        );
        assert!(filename_from_url("file:///c:/x").is_err());
        assert!(filename_from_url("https://user:pass@example.com/x").is_err());
    }
    #[test]
    fn categories() {
        assert_eq!(
            Settings::default().category_for("THING.ZIP", "", 0),
            "Compressed"
        );
        assert_eq!(
            Settings::default().category_for("movie.mp4", "", 0),
            "Video"
        );
    }
    #[test]
    fn column_preferences_preserve_order_visibility_and_width_after_restart() {
        let legacy: Settings = serde_json::from_str("{}").unwrap();
        assert!(legacy.columns.is_empty());
        let temp = TestDirectory::new();
        let mut library = Library::default();
        library.settings.columns = vec![(2, true, 175), (0, false, 230), (1, true, 28)];
        library.save(&temp.library()).unwrap();
        let loaded = Library::load(&temp.library()).unwrap();
        assert_eq!(loaded.settings.columns, library.settings.columns);
    }
    #[test]
    fn parent_page_survives_restart_independently_of_request_referer() {
        let legacy: Download =
            serde_json::from_str(r#"{"referer":"https://headers.example/"}"#).unwrap();
        assert!(legacy.parent_page.is_empty());
        let temp = TestDirectory::new();
        let mut library = Library::default();
        library.downloads.push(Download {
            id: 1,
            parent_page: "https://source.example/final-page".into(),
            referer: "https://headers.example/".into(),
            ..Default::default()
        });
        library.save(&temp.library()).unwrap();
        let loaded = Library::load(&temp.library()).unwrap();
        assert_eq!(
            loaded.downloads[0].parent_page,
            "https://source.example/final-page"
        );
        assert_eq!(loaded.downloads[0].referer, "https://headers.example/");
    }
    #[test]
    fn legacy_queues_keep_weekly_defaults_and_new_schedule_fields_persist() {
        let legacy: Queue =
            serde_json::from_str(r#"{"name":"Night","scheduled":true,"start_hour":23}"#).unwrap();
        assert!(legacy.scheduled);
        assert_eq!(legacy.start_hour, 23);
        assert_eq!(legacy.schedule_date, None);
        assert!(!legacy.start_on_startup);
        assert_eq!(legacy.last_schedule_event, None);

        let temp = TestDirectory::new();
        let mut library = Library::default();
        let queue = &mut library.settings.queues[0];
        queue.schedule_date = NaiveDate::from_ymd_opt(2026, 12, 31);
        queue.scheduled = true;
        queue.last_schedule_event = Some(1065575460);
        library.save(&temp.library()).unwrap();
        let loaded = Library::load(&temp.library()).unwrap();
        let queue = &loaded.settings.queues[0];
        assert_eq!(queue.schedule_date, NaiveDate::from_ymd_opt(2026, 12, 31));
        assert_eq!(queue.last_schedule_event, Some(1065575460));
    }

    #[test]
    fn startup_resumes_only_opted_in_queues_and_preserves_terminal_states() {
        let temp = TestDirectory::new();
        let mut library = Library::default();
        library.settings.queues.push(Queue {
            name: "Night".into(),
            start_on_startup: true,
            ..Default::default()
        });
        library.downloads = [
            ("Main download queue", Status::Paused),
            ("Night", Status::Paused),
            ("Night", Status::Downloading),
            ("Night", Status::Complete),
            ("Night", Status::Error),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (queue, status))| Download {
            id: i as u64 + 1,
            queue: queue.into(),
            status,
            ..Default::default()
        })
        .collect();
        library.save(&temp.library()).unwrap();
        let loaded = Library::load(&temp.library()).unwrap();
        assert!(!loaded.settings.queues[0].active);
        assert!(loaded.settings.queues[1].active);
        assert_eq!(
            loaded
                .downloads
                .iter()
                .map(|d| d.status)
                .collect::<Vec<_>>(),
            vec![
                Status::Paused,
                Status::Queued,
                Status::Queued,
                Status::Complete,
                Status::Error
            ]
        );
    }

    #[test]
    fn renaming_queue_updates_all_references_and_rejects_duplicates_atomically() {
        let temp = TestDirectory::new();
        let mut library = Library::default();
        library.settings.queues.push(Queue {
            name: "Night".into(),
            ..Default::default()
        });
        library.downloads = vec![
            Download {
                id: 1,
                queue: "Night".into(),
                status: Status::Downloading,
                ..Default::default()
            },
            Download {
                id: 2,
                queue: "Night".into(),
                status: Status::Complete,
                ..Default::default()
            },
            Download {
                id: 3,
                ..Default::default()
            },
        ];
        assert!(library.rename_queue(1, " main DOWNLOAD queue ").is_err());
        assert!(library.rename_queue(1, " ").is_err());
        assert!(library.rename_queue(1, "bad\nname").is_err());
        assert!(library.rename_queue(8, "Missing").is_err());
        assert_eq!(library.downloads[0].queue, "Night");
        assert_eq!(library.rename_queue(1, " Overnight ").unwrap(), "Night");
        assert_eq!(library.downloads[0].queue, "Overnight");
        assert_eq!(library.downloads[1].queue, "Overnight");
        assert_eq!(library.downloads[2].queue, "Main download queue");
        library.save(&temp.library()).unwrap();
        let loaded = Library::load(&temp.library()).unwrap();
        assert_eq!(loaded.settings.queues[1].name, "Overnight");
        assert_eq!(loaded.downloads[1].queue, "Overnight");
    }
    #[test]
    fn per_download_preferences_keep_legacy_defaults_and_survive_restart() {
        let legacy: Download = serde_json::from_str(r#"{"id":1}"#).unwrap();
        assert_eq!(legacy.speed_limit_kib, None);
        assert!(legacy.notify_on_complete);
        assert!(!legacy.open_folder_on_complete);
        assert!(!legacy.close_progress_on_complete);

        let directory = TestDirectory::new();
        let library = Library {
            downloads: vec![
                Download {
                    id: 1,
                    speed_limit_kib: Some(0),
                    notify_on_complete: false,
                    open_folder_on_complete: true,
                    close_progress_on_complete: true,
                    ..Default::default()
                },
                Download {
                    id: 2,
                    speed_limit_kib: Some(512),
                    ..Default::default()
                },
                legacy,
            ],
            ..Default::default()
        };
        library.save(&directory.library()).unwrap();
        let restored = Library::load(&directory.library()).unwrap();
        let unlimited = &restored.downloads[0];
        assert_eq!(unlimited.speed_limit_kib, Some(0));
        assert!(!unlimited.notify_on_complete);
        assert!(unlimited.open_folder_on_complete);
        assert!(unlimited.close_progress_on_complete);
        assert_eq!(restored.downloads[1].speed_limit_kib, Some(512));
        assert_eq!(restored.downloads[2].speed_limit_kib, None);
    }
    #[test]
    fn corrupt_library_restores_backup_and_keeps_using_canonical_path() {
        let directory = TestDirectory::new();
        let path = directory.library();
        let mut library = Library {
            version: 1,
            next_id: 2,
            downloads: vec![Download {
                id: 1,
                description: "from backup".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        library.save(&path).unwrap();
        library.downloads[0].description = "latest state".into();
        library.save(&path).unwrap();
        std::fs::write(&path, b"{ broken json evidence").unwrap();
        let mut restored = Library::load(&path).unwrap();
        assert_eq!(restored.downloads[0].description, "from backup");
        assert!(restored.recovery_notice.is_some());
        let preserved: Vec<_> = std::fs::read_dir(&directory.0)
            .unwrap()
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().contains("corrupt-"))
            .collect();
        assert_eq!(preserved.len(), 1);
        assert_eq!(
            std::fs::read(preserved[0].path()).unwrap(),
            b"{ broken json evidence"
        );
        restored.downloads[0].description = "work after recovery".into();
        restored.save(&path).unwrap();
        let reopened = Library::load(&path).unwrap();
        assert_eq!(reopened.downloads[0].description, "work after recovery");
        assert!(reopened.recovery_notice.is_none());
    }
    #[test]
    fn corrupt_library_without_backup_starts_one_persistent_replacement() {
        let directory = TestDirectory::new();
        let path = directory.library();
        std::fs::write(&path, b"damaged").unwrap();
        let mut recovered = Library::load(&path).unwrap();
        assert!(recovered.recovery_notice.is_some());
        recovered.downloads.push(Download {
            id: 1,
            description: "new work".into(),
            ..Default::default()
        });
        recovered.save(&path).unwrap();
        assert_eq!(
            Library::load(&path).unwrap().downloads[0].description,
            "new work"
        );
    }
    #[test]
    fn partial_folders_and_existing_rows_reserve_destinations() {
        let directory = TestDirectory::new();
        let original = directory.0.join("archive.zip");
        std::fs::create_dir(partial_path(&original)).unwrap();
        assert_eq!(
            unique_path(&directory.0, "archive.zip", &[]),
            directory.0.join("archive (1).zip")
        );
        let downloads = vec![Download {
            id: 1,
            path: directory.0.join("archive (1).zip"),
            ..Default::default()
        }];
        assert_eq!(
            unique_path(&directory.0, "archive.zip", &downloads),
            directory.0.join("archive (2).zip")
        );
        assert!(partial_path(&original).is_dir());
    }
    #[test]
    fn verification_state_survives_restart_and_only_verified_content_completes() {
        let directory = TestDirectory::new();
        let file = directory.0.join("test.txt");
        std::fs::write(&file, b"abc").unwrap();
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let mut library = Library {
            version: 1,
            next_id: 2,
            downloads: vec![Download {
                id: 1,
                path: file.clone(),
                status: Status::Verifying,
                sha256: expected.into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        library.save(&directory.library()).unwrap();
        library = Library::load(&directory.library()).unwrap();
        assert_eq!(library.downloads[0].status, Status::Verifying);
        library.downloads[0].finish_verification(verify_sha256(&file, expected));
        assert_eq!(library.downloads[0].status, Status::Complete);
        library.save(&directory.library()).unwrap();
        let reopened = Library::load(&directory.library()).unwrap();
        assert_eq!(reopened.downloads[0].status, Status::Complete);
        assert_eq!(reopened.downloads[0].verified_sha256, expected);
    }
    #[test]
    fn missing_and_mismatched_files_fail_verification() {
        let directory = TestDirectory::new();
        let path = directory.0.join("test.txt");
        let expected = "0".repeat(64);
        let mut download = Download {
            status: Status::Verifying,
            sha256: expected.clone(),
            ..Default::default()
        };
        download.finish_verification(verify_sha256(&path, &expected));
        assert_eq!(download.status, Status::Error);
        assert!(download.error.contains("Could not open"));
        std::fs::write(&path, b"different content").unwrap();
        download.status = Status::Verifying;
        download.finish_verification(verify_sha256(&path, &expected));
        assert_eq!(download.status, Status::Error);
        assert!(download.error.contains("mismatch"));
    }
    #[test]
    fn legacy_complete_entry_with_unverified_checksum_is_rechecked() {
        let directory = TestDirectory::new();
        let library = Library {
            downloads: vec![Download {
                id: 1,
                status: Status::Complete,
                sha256: "a".repeat(64),
                ..Default::default()
            }],
            ..Default::default()
        };
        library.save(&directory.library()).unwrap();
        assert_eq!(
            Library::load(&directory.library()).unwrap().downloads[0].status,
            Status::Verifying
        );
    }
    #[test]
    fn new_downloads_do_not_restart_paused_entries_and_later_stays_paused() {
        let mut library = Library::default();
        let queue = library.settings.queues[0].name.clone();
        library.settings.queues[0].active = true;
        library.downloads = (1..=3)
            .map(|id| Download {
                id,
                queue: queue.clone(),
                status: Status::Paused,
                ..Default::default()
            })
            .collect();
        library.prepare_added(&[2], &queue, false);
        assert_eq!(library.downloads[1].status, Status::Paused);
        library.prepare_added(&[3], &queue, true);
        assert_eq!(library.downloads[0].status, Status::Paused);
        assert_eq!(library.downloads[1].status, Status::Paused);
        assert_eq!(library.downloads[2].status, Status::Queued);
        assert!(library.settings.queues[0].active);
    }
}
