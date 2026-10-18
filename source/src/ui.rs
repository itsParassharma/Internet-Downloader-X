use crate::{
    engine::{self, DownloadControl, DownloadEvent, DownloadSpec, EventKind},
    model::*,
};
use chrono::{Datelike, Local, Timelike, Utc};
use eframe::egui::{self, Color32, RichText, Vec2};
use egui_extras::{Column, TableBuilder};
mod classic;
mod dialogs;
#[cfg(windows)]
mod native_columns;
#[cfg(windows)]
mod native_forms;
#[cfg(windows)]
pub(crate) mod native_main;
#[cfg(windows)]
mod native_progress;
#[cfg(windows)]
mod native_resources;
#[cfg(windows)]
mod native_settings;
#[cfg(not(windows))]
mod progress;
use classic::{style, toolbar_button, tree_row};
use std::{
    collections::{HashMap, HashSet},
    net::TcpListener,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};

const BLUE: Color32 = Color32::from_rgb(33, 104, 174);

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortColumn {
    Filename,
    Size,
    Status,
    Remaining,
    Speed,
    Date,
    Queue,
    Description,
    Added,
    Folder,
    Referer,
    ParentPage,
}
#[derive(Default)]
struct AddForm {
    open: bool,
    urls: String,
    filename: String,
    folder: String,
    queue: usize,
    category: usize,
    exact_destination: bool,
    remember_destination: bool,
    start: bool,
    advanced: bool,
    username: String,
    password: String,
    referer: String,
    parent_page: String,
    parent_pages: HashMap<String, String>,
    browser_requests: HashMap<
        String,
        (
            crate::browser_session::BrowserContext,
            crate::streaming::StreamOptions,
            String,
        ),
    >,
    sha256: String,
    description: String,
    error: String,
    edit_id: Option<u64>,
}
pub struct IdxApp {
    browser_setup_pending: bool,
    art: crate::ui_assets::UiAssets,
    categories_open: bool,
    queues_open: bool,
    show_categories: bool,
    show_search: bool,
    transfer_open: bool,
    #[cfg(windows)]
    native_progress: native_progress::NativeProgress,
    #[cfg(not(windows))]
    progress_show_details: bool,
    #[cfg(not(windows))]
    progress_limit_input: u64,
    lib: Library,
    library_path: PathBuf,
    tx: Sender<DownloadEvent>,
    rx: Receiver<DownloadEvent>,
    controls: HashMap<u64, DownloadControl>,
    selected: Option<u64>,
    selected_ids: HashSet<u64>,
    selection_anchor: Option<u64>,
    select_all_requested: bool,
    sort_column: Option<SortColumn>,
    sort_descending: bool,
    filter: String,
    search: String,
    add: AddForm,
    options: bool,
    options_tab: usize,
    scheduler: bool,
    scheduler_tab: usize,
    about: bool,
    help: bool,
    grabber: bool,
    message: Option<String>,
    delete_ids: Vec<u64>,
    #[cfg(not(windows))]
    detail_tab: usize,
    last_save: Instant,
    last_inbox: Instant,
    last_schedule: Option<i64>,
    listener: TcpListener,
    queue_index: usize,
    new_queue: String,
    grab_url: String,
    grab_filter: String,
    grab_links: Vec<(bool, String)>,
    grab_rx: Option<Receiver<crate::grabber::ScanEvent>>,
    grab_control: Option<crate::grabber::ScanHandle>,
    grab_parents: HashMap<String, String>,
    grab_status: String,
    grab_page: String,
    grab_error: String,
    hash_tx: Sender<(u64, Result<String, String>)>,
    hash_rx: Receiver<(u64, Result<String, String>)>,
    hashing: HashSet<u64>,
    closing: bool,
    persistence_blocked: bool,
    #[cfg(windows)]
    focus_requested: bool,
    #[cfg(windows)]
    pending_completions: Vec<u64>,
}
impl IdxApp {
    #[cfg(not(windows))]
    pub fn new(cc: &eframe::CreationContext<'_>, path: PathBuf, listener: TcpListener) -> Self {
        Self::from_context(&cc.egui_ctx, path, listener)
    }
    fn from_context(ctx: &egui::Context, path: PathBuf, listener: TcpListener) -> Self {
        let (lib, message, persistence_blocked) = match Library::load(&path) {
            Ok(mut l) => {
                let notice = l.recovery_notice.take();
                (l, notice, false)
            }
            Err(e) => (
                Library {
                    version: 1,
                    next_id: 1,
                    ..Default::default()
                },
                Some(e),
                true,
            ),
        };
        classic::install_font(ctx);
        style(ctx, lib.settings.dark);
        let (tx, rx) = mpsc::channel();
        let (hash_tx, hash_rx) = mpsc::channel();
        Self {
            browser_setup_pending: !crate::browser_setup::load(
                path.parent().unwrap_or(Path::new(".")),
            )
            .offered,
            art: crate::ui_assets::UiAssets::new(ctx),
            categories_open: true,
            queues_open: true,
            show_categories: true,
            show_search: false,
            transfer_open: false,
            #[cfg(windows)]
            native_progress: native_progress::NativeProgress::new(),
            #[cfg(not(windows))]
            progress_show_details: true,
            #[cfg(not(windows))]
            progress_limit_input: 512,
            lib,
            library_path: path,
            tx,
            rx,
            controls: HashMap::new(),
            selected: None,
            selected_ids: HashSet::new(),
            selection_anchor: None,
            select_all_requested: false,
            sort_column: None,
            sort_descending: false,
            filter: "All downloads".into(),
            search: String::new(),
            add: AddForm::default(),
            options: false,
            options_tab: 0,
            scheduler: false,
            scheduler_tab: 0,
            about: false,
            help: false,
            grabber: false,
            message,
            delete_ids: vec![],
            #[cfg(not(windows))]
            detail_tab: 0,
            last_save: Instant::now(),
            last_inbox: Instant::now() - Duration::from_secs(3),
            last_schedule: None,
            listener,
            queue_index: 0,
            new_queue: String::new(),
            grab_url: String::new(),
            grab_filter: String::new(),
            grab_links: vec![],
            grab_rx: None,
            grab_control: None,
            grab_parents: HashMap::new(),
            grab_status: String::new(),
            grab_page: String::new(),
            grab_error: String::new(),
            hash_tx,
            hash_rx,
            hashing: HashSet::new(),
            closing: false,
            persistence_blocked,
            #[cfg(windows)]
            focus_requested: false,
            #[cfg(windows)]
            pending_completions: Vec::new(),
        }
    }
    fn save(&mut self) {
        if self.persistence_blocked {
            return;
        }
        if let Err(e) = self.lib.save(&self.library_path) {
            self.message = Some(format!("Could not save the library: {e}"));
        }
        self.last_save = Instant::now();
    }
    fn add_dialog(&mut self, urls: String) {
        self.add = AddForm {
            open: true,
            urls,
            folder: self.lib.settings.download_dir.to_string_lossy().into(),
            start: true,
            ..Default::default()
        };
    }
    fn start(&mut self, id: u64) {
        if self.closing || self.controls.contains_key(&id) {
            return;
        }
        let s = &self.lib.settings;
        let Some(d) = self.lib.downloads.iter_mut().find(|d| d.id == id) else {
            return;
        };
        if matches!(d.status, Status::Complete | Status::Verifying) {
            return;
        }
        if d.path.exists() {
            d.status = Status::Error;
            d.error =
                "The destination already exists. Edit this download to choose another filename."
                    .into();
            return;
        }
        d.status = Status::Connecting;
        d.error.clear();
        d.speed = 0.0;
        d.last_try = Some(Utc::now());
        d.note("Connecting to the download server.");
        let spec = DownloadSpec {
            browser: d.browser.clone(),
            stream: d.stream.clone(),
            id: d.id,
            url: d.url.clone(),
            destination: d.path.clone(),
            connections: s.connections,
            retries: s.retries,
            speed_limit_kib: d.speed_limit_kib.unwrap_or(s.speed_limit_kib),
            username: d.username.clone(),
            password: d.password.clone(),
            referer: d.referer.clone(),
            user_agent: s.user_agent.clone(),
            proxy: s.proxy.clone(),
        };
        self.controls
            .insert(id, engine::spawn(spec, self.tx.clone()));
        if self.selected.is_none() {
            self.selected = Some(id);
            self.selected_ids.insert(id);
            self.selection_anchor = Some(id);
        }
    }
    fn pause(&mut self, id: u64) {
        if let Some(c) = self.controls.get(&id) {
            c.pause();
        } else if let Some(d) = self.lib.downloads.iter_mut().find(|d| d.id == id) {
            if d.status == Status::Queued {
                d.status = Status::Paused;
            }
        }
    }
    fn pause_all(&mut self) {
        for q in &mut self.lib.settings.queues {
            q.active = false;
        }
        let ids: Vec<_> = self.controls.keys().copied().collect();
        for id in ids {
            self.pause(id);
        }
    }
    fn start_queue(&mut self, index: usize) {
        if self.closing {
            return;
        }
        if let Some(q) = self.lib.settings.queues.get_mut(index) {
            q.active = true;
            let name = q.name.clone();
            for d in &mut self.lib.downloads {
                if d.queue == name && d.status == Status::Paused {
                    d.status = Status::Queued;
                }
            }
        }
    }
    fn stop_queue(&mut self, index: usize) {
        if let Some(q) = self.lib.settings.queues.get_mut(index) {
            q.active = false;
            let name = q.name.clone();
            let ids: Vec<_> = self
                .lib
                .downloads
                .iter()
                .filter(|d| d.queue == name && d.status.active())
                .map(|d| d.id)
                .collect();
            for id in ids {
                self.pause(id);
            }
        }
    }
    fn rename_queue(&mut self, index: usize, name: &str) -> Result<(), String> {
        let old = self.lib.rename_queue(index, name)?;
        if self.filter == format!("queue:{old}") {
            self.filter = format!("queue:{}", name.trim());
        }
        self.save();
        Ok(())
    }
    fn selected_targets(&self) -> Vec<u64> {
        self.lib
            .downloads
            .iter()
            .filter(|download| {
                self.selected_ids.contains(&download.id)
                    || (self.selected_ids.is_empty() && self.selected == Some(download.id))
            })
            .map(|download| download.id)
            .collect()
    }
    fn resume_selected(&mut self) {
        for id in self.selected_targets() {
            self.start(id);
        }
    }
    fn pause_selected(&mut self) {
        for id in self.selected_targets() {
            self.pause(id);
        }
    }
    fn request_delete(&mut self) {
        let ids = self.selected_targets();
        if ids.is_empty() {
            return;
        }
        if self.lib.settings.confirm_delete {
            self.delete_ids = ids;
        } else {
            self.remove_many(&ids);
        }
    }
    fn remove_many(&mut self, ids: &[u64]) {
        let mut blocked = 0;
        let removed: HashSet<_> = self
            .lib
            .downloads
            .iter()
            .filter(|download| ids.contains(&download.id))
            .filter_map(|download| {
                if self.controls.contains_key(&download.id) || download.status == Status::Verifying
                {
                    blocked += 1;
                    None
                } else {
                    Some(download.id)
                }
            })
            .collect();
        self.lib
            .downloads
            .retain(|download| !removed.contains(&download.id));
        self.selected_ids.retain(|id| !removed.contains(id));
        if self.selected.is_some_and(|id| removed.contains(&id)) {
            self.selected = self
                .lib
                .downloads
                .iter()
                .find(|download| self.selected_ids.contains(&download.id))
                .map(|download| download.id);
        }
        if self
            .selection_anchor
            .is_some_and(|id| removed.contains(&id))
        {
            self.selection_anchor = self.selected;
        }
        if blocked > 0 {
            self.message = Some(format!(
                "{blocked} selected download(s) are still running or being verified. Pause transfers and wait for verification before removing those entries."
            ));
        }
        self.save();
    }
    fn edit(&mut self, id: u64) {
        if let Some(d) = self.lib.downloads.iter().find(|d| d.id == id) {
            if d.status.active() || d.status == Status::Verifying {
                self.message = Some("Pause an active download or wait for checksum verification before editing its properties.".into());
                return;
            }
            self.add = AddForm {
                open: true,
                urls: d.url.clone(),
                filename: d.name(),
                folder: d
                    .path
                    .parent()
                    .unwrap_or(Path::new(""))
                    .to_string_lossy()
                    .into(),
                queue: self
                    .lib
                    .settings
                    .queues
                    .iter()
                    .position(|q| q.name == d.queue)
                    .unwrap_or(0),
                category: self
                    .lib
                    .settings
                    .categories
                    .iter()
                    .position(|c| c.name == d.category)
                    .map(|i| i + 1)
                    .unwrap_or(0),
                start: false,
                advanced: true,
                username: d.username.clone(),
                password: d.password.clone(),
                referer: d.referer.clone(),
                sha256: d.sha256.clone(),
                description: d.description.clone(),
                edit_id: Some(id),
                ..Default::default()
            };
        }
    }
    fn tick(&mut self, ctx: &egui::Context) {
        let mut state_changed = false;
        let mut completed = Vec::new();
        while let Ok(e) = self.rx.try_recv() {
            let terminal = matches!(
                e.kind,
                EventKind::Completed { .. } | EventKind::Failed(_) | EventKind::Paused
            );
            if terminal {
                self.controls.remove(&e.id);
                state_changed = true;
            }
            if let Some(d) = self.lib.downloads.iter_mut().find(|d| d.id == e.id) {
                match e.kind {
                    EventKind::Started { total, resumable } => {
                        d.total = total;
                        d.resumable = resumable;
                        d.status = Status::Downloading;
                        d.note(if resumable{"Server supports resumable transfers."}else{"Server does not support safe resume; interrupted transfers will restart."});
                    }
                    EventKind::Progress {
                        downloaded,
                        total,
                        speed,
                        parts,
                    } => {
                        d.downloaded = downloaded;
                        d.total = total;
                        d.speed = speed;
                        d.parts = parts.into_iter().map(|p| (p.downloaded, p.total)).collect();
                    }
                    EventKind::Completed { path } => {
                        d.path = path.clone();
                        d.speed = 0.0;
                        d.total = std::fs::metadata(&path).ok().map(|m| m.len());
                        d.downloaded = d.total.unwrap_or(d.downloaded);
                        d.verified_sha256.clear();
                        if !d.sha256.is_empty() {
                            d.status = Status::Verifying;
                            d.note("Verifying SHA-256 checksum…");
                        } else {
                            d.status = Status::Complete;
                            d.note("Download complete.");
                            completed.push(d.id);
                        }
                    }
                    EventKind::Paused => {
                        d.status = Status::Paused;
                        d.speed = 0.0;
                        d.note("Paused. Resume data retained.");
                    }
                    EventKind::Failed(error) => {
                        d.status = Status::Error;
                        d.speed = 0.0;
                        d.error = error.clone();
                        d.note(error);
                    }
                }
            }
        }
        while let Ok((id, result)) = self.hash_rx.try_recv() {
            self.hashing.remove(&id);
            state_changed = true;
            if let Some(d) = self.lib.downloads.iter_mut().find(|d| d.id == id) {
                d.finish_verification(result);
                if d.status == Status::Complete {
                    completed.push(d.id);
                }
            }
        }
        let mut notices = Vec::new();
        for id in completed {
            if let Some(d) = self.lib.downloads.iter().find(|d| d.id == id) {
                if d.open_folder_on_complete {
                    if let Some(folder) = d.path.parent() {
                        open_path(folder);
                    }
                }
                if d.close_progress_on_complete && self.selected == Some(id) {
                    self.transfer_open = false;
                }
                if d.notify_on_complete {
                    notices.push(d.name());
                    #[cfg(windows)]
                    self.pending_completions.push(d.id);
                }
            }
        }
        #[cfg(not(windows))]
        if !notices.is_empty() {
            self.message = Some(format!("Download complete:\n{}", notices.join("\n")));
        }
        if state_changed {
            self.save();
        }
        // Closing waits for download workers; unfinished hashes are persisted
        // as Verifying and resumed when the application is opened again.
        if self.closing {
            return;
        }
        for download in &self.lib.downloads {
            if download.status == Status::Verifying && self.hashing.insert(download.id) {
                let (id, path, expected, sender) = (
                    download.id,
                    download.path.clone(),
                    download.sha256.clone(),
                    self.hash_tx.clone(),
                );
                std::thread::spawn(move || {
                    let result = verify_sha256(&path, &expected);
                    let _ = sender.send((id, result));
                });
            }
        }
        if self.last_inbox.elapsed() > Duration::from_secs(1) {
            self.last_inbox = Instant::now();
            if let Ok((mut stream, _)) = self.listener.accept() {
                use std::io::Read;
                stream.set_read_timeout(Some(Duration::from_millis(5))).ok();
                let mut bytes = [0u8; 16];
                let _ = stream.read(&mut bytes);
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                #[cfg(windows)]
                {
                    self.focus_requested = true;
                }
            }
            let inbox = crate::app_data().join("inbox");
            if let Ok(entries) = std::fs::read_dir(inbox) {
                let mut incoming = vec![];
                let mut incoming_parents = HashMap::new();
                let mut browser_requests = HashMap::new();
                for entry in entries.flatten().take(100) {
                    if entry.path().extension().is_some_and(|x| x == "json") {
                        if let Ok(bytes) = std::fs::read(entry.path()) {
                            if let Ok(request) =
                                serde_json::from_slice::<crate::external::Request>(&bytes)
                            {
                                let context = match request.browser_context() {
                                    Ok(c) => c,
                                    Err(e) => {
                                        self.message = Some(e);
                                        continue;
                                    }
                                };
                                if request.automatic {
                                    let key = entry.file_name().to_string_lossy().into_owned();
                                    match self.accept_browser_download_with_session(
                                        &request.url,
                                        &key,
                                        &request.parent_page,
                                        &request.filename,
                                        &request.referer,
                                        context,
                                        request.stream.clone(),
                                    ) {
                                        Ok(()) => {
                                            std::fs::remove_file(entry.path()).ok();
                                        }
                                        Err(error) => {
                                            self.message = Some(format!(
                                                "Browser download is still queued: {error}"
                                            ))
                                        }
                                    }
                                } else if filename_from_url(&request.url).is_ok() {
                                    incoming_parents
                                        .insert(request.url.clone(), request.parent_page);
                                    browser_requests.insert(
                                        request.url.clone(),
                                        (context, request.stream, request.referer),
                                    );
                                    incoming.push(request.url);
                                    std::fs::remove_file(entry.path()).ok();
                                }
                            } else if let Ok(urls) = serde_json::from_slice::<Vec<String>>(&bytes) {
                                incoming.extend(
                                    urls.into_iter().filter(|s| filename_from_url(s).is_ok()),
                                );
                                std::fs::remove_file(entry.path()).ok();
                            }
                        }
                    }
                }
                if !incoming.is_empty() {
                    let urls = incoming.join("\n");
                    if self.add.open {
                        self.add.urls.push('\n');
                        self.add.urls.push_str(&urls);
                    } else {
                        self.add_dialog(urls);
                    }
                    self.add.parent_pages.extend(incoming_parents);
                    self.add.browser_requests.extend(browser_requests);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    #[cfg(windows)]
                    {
                        self.focus_requested = true;
                    }
                }
            }
        }
        let now = Local::now();
        let moment = crate::schedule::Moment {
            day: now.date_naive().num_days_from_ce(),
            weekday: now.weekday().num_days_from_monday(),
            minute: now.hour() * 60 + now.minute(),
        };
        if self
            .last_schedule
            .is_none_or(|previous| moment.key() > previous)
        {
            let events: Vec<_> = self
                .lib
                .settings
                .queues
                .iter()
                .enumerate()
                .filter_map(|(i, q)| {
                    let schedule = crate::schedule::Schedule {
                        enabled: q.scheduled,
                        start_minute: q
                            .start_hour
                            .saturating_mul(60)
                            .saturating_add(q.start_minute),
                        stop_minute: q
                            .stop_enabled
                            .then(|| q.stop_hour.saturating_mul(60).saturating_add(q.stop_minute)),
                        weekdays: q.weekdays,
                        once_day: q.schedule_date.map(|date| date.num_days_from_ce()),
                    };
                    crate::schedule::due(
                        schedule,
                        moment,
                        self.last_schedule,
                        q.last_schedule_event,
                    )
                    .map(|event| (i, event))
                })
                .collect();
            self.last_schedule = Some(moment.key());
            for (i, event) in &events {
                self.lib.settings.queues[*i].last_schedule_event = Some(event.minute);
                match event.action {
                    crate::schedule::Action::Start => self.start_queue(*i),
                    crate::schedule::Action::Stop => self.stop_queue(*i),
                }
            }
            if !events.is_empty() {
                self.save();
            }
        }
        let mut starts = vec![];
        for q in &self.lib.settings.queues {
            if q.active {
                let active = self
                    .lib
                    .downloads
                    .iter()
                    .filter(|d| d.queue == q.name && d.status.active())
                    .count();
                starts.extend(
                    self.lib
                        .downloads
                        .iter()
                        .filter(|d| d.queue == q.name && d.status == Status::Queued)
                        .take(q.parallel.saturating_sub(active))
                        .map(|d| d.id),
                );
            }
        }
        for id in starts {
            self.start(id);
        }
        if self.last_save.elapsed() > Duration::from_secs(3) {
            self.save();
        }
        self.poll_grabber();
    }
    fn start_grabber(&mut self, options: crate::grabber::ScanOptions) {
        self.cancel_grabber();
        self.grab_links.clear();
        self.grab_parents.clear();
        self.grab_page.clear();
        self.grab_error.clear();
        self.grab_status = "Connecting...".into();
        let (tx, rx) = mpsc::channel();
        self.grab_control = Some(crate::grabber::scan(self.grab_url.clone(), options, tx));
        self.grab_rx = Some(rx);
    }
    fn cancel_grabber(&mut self) {
        self.grab_control = None;
        self.grab_rx = None;
    }
    fn poll_grabber(&mut self) {
        use crate::grabber::ScanEvent;
        loop {
            let Some(rx) = &self.grab_rx else { break };
            match rx.try_recv() {
                Ok(ScanEvent::Progress { pages, links }) => {
                    self.grab_status = format!("Scanning: {pages} pages, {links} links");
                }
                Ok(ScanEvent::Checking { checked, total }) => {
                    self.grab_status = format!("Checking file sizes: {checked} of {total}");
                }
                Ok(ScanEvent::Finished(result)) => {
                    match result {
                        Ok(links) => {
                            self.grab_status = links.summary();
                            self.grab_page = links.page_url;
                            self.grab_parents = links.parents.into_iter().collect();
                            self.grab_links = links.urls.into_iter().map(|u| (false, u)).collect();
                        }
                        Err(error) => {
                            self.grab_error = error;
                            self.grab_status.clear();
                        }
                    }
                    self.cancel_grabber();
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.grab_error =
                        "The page scanner stopped unexpectedly. Try scanning again.".into();
                    self.cancel_grabber();
                    break;
                }
            }
        }
    }
    fn import(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("URL lists", &["txt", "json"])
            .pick_file()
        {
            match std::fs::read_to_string(path) {
                Ok(s) => {
                    let urls = serde_json::from_str::<Vec<String>>(&s)
                        .map(|u| u.join("\n"))
                        .unwrap_or(s);
                    self.add_dialog(urls);
                }
                Err(e) => self.message = Some(e.to_string()),
            }
        }
    }
    fn export(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("idx-downloads.txt")
            .save_file()
        {
            let content = self
                .lib
                .downloads
                .iter()
                .map(|d| d.url.as_str())
                .collect::<Vec<_>>()
                .join("\r\n");
            if let Err(e) = std::fs::write(path, content) {
                self.message = Some(e.to_string());
            }
        }
    }
    fn accept_add(&mut self) {
        self.add.error.clear();
        let urls = match crate::batch::expand_urls(&self.add.urls) {
            Ok(urls) => urls,
            Err(error) => {
                self.add.error = error;
                return;
            }
        };
        if self.add.edit_id.is_some() && urls.len() != 1 {
            self.add.error =
                "A download can have only one address. Use Add URL for a batch.".into();
            return;
        }
        let single_url = urls.len() == 1;
        let names: Result<Vec<_>, _> = urls.iter().map(|u| filename_from_url(u)).collect();
        let names = match names {
            Ok(n) => n,
            Err(e) => {
                self.add.error = e;
                return;
            }
        };
        if !self.add.sha256.is_empty()
            && (self.add.sha256.len() != 64
                || !self.add.sha256.bytes().all(|b| b.is_ascii_hexdigit())
                || urls.len() != 1)
        {
            self.add.error =
                "SHA-256 must be 64 hexadecimal characters for a single download.".into();
            return;
        }
        let dir = PathBuf::from(self.add.folder.trim());
        if !dir.is_absolute() {
            self.add.error = "Choose an absolute destination folder.".into();
            return;
        }
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.add.error = format!("Cannot create folder: {e}");
            return;
        }
        let queue = self
            .lib
            .settings
            .queues
            .get(self.add.queue)
            .map(|q| q.name.clone())
            .unwrap_or_else(|| self.lib.settings.queues[0].name.clone());
        let destination_settings = self.lib.settings.clone();
        let mut ids = vec![];
        for (url, name) in urls.into_iter().zip(names) {
            let name = if !self.add.filename.trim().is_empty() && single_url {
                safe_filename(&self.add.filename)
            } else {
                name
            };
            let category = destination_settings.category_for(&name, &url, self.add.category);
            let exact = self.add.exact_destination || self.add.edit_id.is_some();
            let base = destination_settings.destination_folder(&category, &dir, exact);
            if let Some(id) = self.add.edit_id {
                let Some(previous) = self.lib.downloads.iter().find(|d| d.id == id) else {
                    return;
                };
                if previous.status.active()
                    || matches!(previous.status, Status::Complete | Status::Verifying)
                {
                    self.add.error = "Pause an active download and wait for verification before editing it. Completed downloads cannot be edited.".into();
                    return;
                }
                let changed_url = previous.url != url;
                let next = if changed_url {
                    unique_path(&base, &name, &self.lib.downloads)
                } else {
                    base.join(&name)
                };
                if next != previous.path
                    && (next.exists()
                        || partial_path(&next).exists()
                        || self.lib.downloads.iter().any(|other| {
                            other.id != id
                                && other
                                    .path
                                    .to_string_lossy()
                                    .eq_ignore_ascii_case(&next.to_string_lossy())
                        }))
                {
                    self.add.error =
                        "That destination is already used by another file or partial download."
                            .into();
                    return;
                }
                if let Some(d) = self.lib.downloads.iter_mut().find(|d| d.id == id) {
                    if d.url != url || d.path != next {
                        d.downloaded = 0;
                        d.total = None;
                        d.parts.clear();
                    }
                    d.url = url;
                    d.path = next;
                    d.category = category;
                    d.queue = queue.clone();
                    d.username = self.add.username.clone();
                    d.password = self.add.password.clone();
                    d.referer = self.add.referer.clone();
                    d.sha256 = self.add.sha256.clone();
                    d.verified_sha256.clear();
                    d.description = self.add.description.clone();
                    d.status = Status::Paused;
                    d.error.clear();
                    if changed_url {
                        d.note("The address changed. A fresh destination was reserved; any previous partial data remains on disk.");
                    }
                    if let Some((browser, stream, referer)) = self
                        .add
                        .browser_requests
                        .get(&self.lib.downloads.last().unwrap().url)
                    {
                        let d = self.lib.downloads.last_mut().unwrap();
                        d.browser = browser.clone();
                        d.stream = stream.clone();
                        if d.referer.is_empty() {
                            d.referer = referer.clone();
                        }
                    }
                    ids.push(id);
                }
                break;
            }
            let path = unique_path(&base, &name, &self.lib.downloads);
            if (exact && self.add.remember_destination)
                || (!exact && dir != destination_settings.download_dir)
            {
                self.lib
                    .settings
                    .remember_category_folder(&category, &base, false);
            }
            let parent_page = self
                .add
                .parent_pages
                .get(&url)
                .unwrap_or(&self.add.parent_page)
                .clone();
            let id = self.lib.next_id;
            self.lib.next_id += 1;
            self.lib.downloads.push(Download {
                id,
                url,
                path,
                category,
                queue: queue.clone(),
                username: self.add.username.clone(),
                password: self.add.password.clone(),
                referer: self.add.referer.clone(),
                parent_page,
                sha256: self.add.sha256.clone(),
                description: self.add.description.clone(),
                ..Default::default()
            });
            ids.push(id);
        }
        self.add.open = false;
        self.add.password.clear();
        self.add.browser_requests.clear();
        self.selected = ids.first().copied();
        self.selected_ids = ids.iter().copied().collect();
        self.selection_anchor = self.selected;
        self.lib.prepare_added(&ids, &queue, self.add.start);
        if self.add.start {
            self.transfer_open = true;
        }
        self.save();
    }
    #[cfg(test)]
    fn accept_browser_download(
        &mut self,
        url: &str,
        external_id: &str,
        parent_page: &str,
        filename: &str,
        referer: &str,
    ) -> Result<(), String> {
        self.accept_browser_download_with_session(
            url,
            external_id,
            parent_page,
            filename,
            referer,
            Default::default(),
            Default::default(),
        )
    }
    fn accept_browser_download_with_session(
        &mut self,
        url: &str,
        external_id: &str,
        parent_page: &str,
        filename: &str,
        referer: &str,
        browser: crate::browser_session::BrowserContext,
        stream: crate::streaming::StreamOptions,
    ) -> Result<(), String> {
        browser.validate()?;
        stream.validate()?;
        crate::browser_media::validate_filename(filename)?;
        if self.persistence_blocked {
            return Err(
                "The download library cannot be saved. Resolve the library error first.".into(),
            );
        }
        if !self
            .lib
            .downloads
            .iter()
            .any(|d| d.external_id == external_id)
        {
            let url_name = filename_from_url(url)?;
            let name = if filename.is_empty() {
                url_name
            } else {
                filename.to_owned()
            };
            let category = self.lib.settings.category_for(&name, url, 0);
            let base = self.lib.settings.category_folder(&category);
            let path = unique_path(&base, &name, &self.lib.downloads);
            let id = self.lib.next_id;
            self.lib.next_id += 1;
            let queue = self
                .lib
                .settings
                .queues
                .first()
                .map(|q| q.name.clone())
                .unwrap_or_else(|| "Main download queue".into());
            self.lib.downloads.push(Download {
                id,
                url: url.into(),
                path,
                category,
                queue: queue.clone(),
                external_id: external_id.into(),
                browser,
                stream,
                parent_page: parent_page.into(),
                referer: referer.into(),
                ..Default::default()
            });
            self.lib.prepare_added(&[id], &queue, true);
        }
        self.lib.save(&self.library_path)
    }
    fn menu_toolbar(&mut self, ctx: &egui::Context) {
        let mut action = 0;
        egui::TopBottomPanel::top("menu").show(ctx,|ui|{
            egui::MenuBar::new().ui(ui,|ui|{
                ui.menu_button("Tasks",|ui|{
                    if ui.button("Add new download…       Ctrl+N").clicked(){action=1;ui.close();}
                    if ui.button("Add batch of URLs…        Ctrl+B").clicked(){action=1;ui.close();}
                    if ui.button("Import URL list…").clicked(){action=2;ui.close();}
                    if ui.button("Export download URLs…").clicked(){action=3;ui.close();}
                    ui.separator();if ui.button("Exit").clicked(){ctx.send_viewport_cmd(egui::ViewportCommand::Close);}
                });
                ui.menu_button("File",|ui|{
                    if ui.button("Open downloaded file").clicked(){action=4;ui.close();}
                    if ui.button("Open containing folder").clicked(){action=5;ui.close();}
                    if ui.button("Properties…").clicked(){action=6;ui.close();}
                });
                ui.menu_button("Downloads",|ui|{
                    if ui.button("Resume selected").clicked(){action=7;ui.close();}
                    if ui.button("Pause selected").clicked(){action=8;ui.close();}
                    if ui.button("Pause all").clicked(){action=9;ui.close();}
                    ui.separator();if ui.button("Start main queue").clicked(){action=10;ui.close();}
                    if ui.button("Stop main queue").clicked(){action=11;ui.close();}
                    if ui.button("Scheduler…").clicked(){self.scheduler=true;ui.close();}
                    ui.separator();if ui.button("Remove completed from list").clicked(){self.lib.downloads.retain(|d|d.status!=Status::Complete);self.selected=None;self.selected_ids.clear();self.selection_anchor=None;ui.close();}
                    if ui.button("Options…").clicked(){self.options=true;ui.close();}
                });
                ui.menu_button("View",|ui|{
                    if ui.selectable_label(!self.lib.settings.dark,"Light interface").clicked(){self.lib.settings.dark=false;style(ctx,false);ui.close();}
                    if ui.selectable_label(self.lib.settings.dark,"Dark interface").clicked(){self.lib.settings.dark=true;style(ctx,true);ui.close();}
                    ui.separator();ui.checkbox(&mut self.show_categories,"Categories");ui.checkbox(&mut self.show_search,"Find downloads");if ui.button("Download progress").clicked(){self.transfer_open=true;ui.close();}
                    ui.separator();if ui.button("Show all downloads").clicked(){self.filter="All downloads".into();self.search.clear();ui.close();}
                });
                ui.menu_button("Help",|ui|{
                    if ui.button("Getting started").clicked(){self.help=true;ui.close();}
                    if ui.button("Browser integration").clicked(){self.message=Some("Install the IDX browser extension, copy its ID from Connection setup, then run the IDX installer and select browser integration. See BROWSER-INTEGRATION.md for details. Automatic downloads start turned off.".into());ui.close();}
                    if ui.button("About IDX").clicked(){self.about=true;ui.close();}
                });
            });
        });
        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .fill(if self.lib.settings.dark {
                        Color32::from_gray(34)
                    } else {
                        Color32::WHITE
                    })
                    .inner_margin(egui::Margin::symmetric(5, 2))
                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(180))),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.horizontal(|ui| {
                        let selected: Vec<_> = self
                            .lib
                            .downloads
                            .iter()
                            .filter(|d| {
                                self.selected_ids.contains(&d.id) || self.selected == Some(d.id)
                            })
                            .collect();
                        let can_resume = selected.iter().any(|d| {
                            !d.status.active()
                                && !matches!(d.status, Status::Complete | Status::Verifying)
                        });
                        let can_pause = selected
                            .iter()
                            .any(|d| d.status.active() || d.status == Status::Queued);
                        for (label, index, enabled, a) in [
                            ("Add URL", 0, true, 1),
                            ("Resume", 1, can_resume, 7),
                            ("Stop", 2, can_pause, 8),
                            ("Stop All", 3, !self.controls.is_empty(), 9),
                            ("Delete", 4, !selected.is_empty(), 12),
                            (
                                "Delete Completed",
                                5,
                                self.lib
                                    .downloads
                                    .iter()
                                    .any(|d| d.status == Status::Complete),
                                16,
                            ),
                            ("Options", 6, true, 13),
                            ("Scheduler", 7, true, 14),
                            ("Start Queue", 8, true, 10),
                            ("Stop Queue", 9, true, 11),
                            ("Grabber", 10, true, 15),
                        ] {
                            if index == 4 || index == 6 {
                                ui.separator();
                            }
                            if toolbar_button(ui, &self.art, label, index, enabled) {
                                action = a;
                            }
                        }
                    });
                });
            });
        match action {
            1 => self.add_dialog(String::new()),
            2 => self.import(),
            3 => self.export(),
            4 => {
                if let Some(d) = self
                    .selected
                    .and_then(|id| self.lib.downloads.iter().find(|d| d.id == id))
                {
                    if d.status == Status::Complete {
                        open_path(&d.path);
                    } else {
                        self.message = Some("The selected download is not complete.".into());
                    }
                }
            }
            5 => {
                if let Some(d) = self
                    .selected
                    .and_then(|id| self.lib.downloads.iter().find(|d| d.id == id))
                {
                    if let Some(p) = d.path.parent() {
                        open_path(p);
                    }
                }
            }
            6 => {
                if let Some(id) = self.selected {
                    self.edit(id);
                }
            }
            7 => self.resume_selected(),
            8 => self.pause_selected(),
            9 => self.pause_all(),
            10 => self.start_queue(0),
            11 => self.stop_queue(0),
            12 => self.request_delete(),
            13 => self.options = true,
            14 => self.scheduler = true,
            15 => self.grabber = true,
            16 => {
                self.lib.downloads.retain(|d| d.status != Status::Complete);
                self.selected = None;
                self.selected_ids.clear();
                self.selection_anchor = None;
                self.save();
            }
            _ => (),
        }
    }
    fn sidebar(&mut self, ctx: &egui::Context) {
        if !self.show_categories {
            return;
        }
        egui::SidePanel::left("categories_classic")
            .default_width(180.0)
            .width_range(140.0..=320.0)
            .resizable(true)
            .frame(
                egui::Frame::new()
                    .fill(if self.lib.settings.dark {
                        Color32::from_gray(27)
                    } else {
                        Color32::WHITE
                    })
                    .inner_margin(3),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Categories");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("×").clicked() {
                            self.show_categories = false;
                        }
                    });
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let (response, toggle) = tree_row(
                        ui,
                        self.art.sidebar("all"),
                        "All Downloads",
                        self.filter == "All downloads",
                        0,
                        Some(self.categories_open),
                    );
                    if toggle {
                        self.categories_open = !self.categories_open;
                    } else if response.clicked() {
                        self.filter = "All downloads".into();
                    }
                    if self.categories_open {
                        for category in &self.lib.settings.categories {
                            let category = category.name.as_str();
                            let key = match category {
                                "Compressed" => "compressed",
                                "Documents" => "documents",
                                "Music" => "music",
                                "Programs" => "programs",
                                "Video" => "video",
                                _ => "other",
                            };
                            if tree_row(
                                ui,
                                self.art.sidebar(key),
                                category,
                                self.filter == category,
                                1,
                                None,
                            )
                            .0
                            .clicked()
                            {
                                self.filter = category.into();
                            }
                        }
                    }
                    for (name, key) in [("Unfinished", "unfinished"), ("Finished", "finished")] {
                        if tree_row(
                            ui,
                            self.art.sidebar(key),
                            name,
                            self.filter == name,
                            0,
                            None,
                        )
                        .0
                        .clicked()
                        {
                            self.filter = name.into();
                        }
                    }
                    if tree_row(ui, self.art.sidebar("grabber"), "Grabber", false, 0, None)
                        .0
                        .clicked()
                    {
                        self.grabber = true;
                    }
                    let (response, toggle) = tree_row(
                        ui,
                        self.art.sidebar("queues"),
                        "Queues",
                        false,
                        0,
                        Some(self.queues_open),
                    );
                    if toggle || response.double_clicked() {
                        self.queues_open = !self.queues_open;
                    }
                    if self.queues_open {
                        for q in &self.lib.settings.queues {
                            let key = format!("queue:{}", q.name);
                            let (response, _) = tree_row(
                                ui,
                                self.art.sidebar("queues"),
                                &q.name,
                                self.filter == key,
                                1,
                                None,
                            );
                            if response.clicked() {
                                self.filter = key;
                            }
                            if response.double_clicked() {
                                self.scheduler = true;
                            }
                        }
                    }
                });
            });
    }
    fn list(&mut self, ctx: &egui::Context) {
        let mut row_action = None;
        let mut row_selection = None;
        let mut visible_ids = Vec::new();
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(if self.lib.settings.dark {
                        Color32::from_gray(27)
                    } else {
                        Color32::WHITE
                    })
                    .inner_margin(0),
            )
            .show(ctx, |ui| {
                if self.show_search {
                    ui.horizontal(|ui| {
                        ui.label("Find:");
                        ui.add(egui::TextEdit::singleline(&mut self.search).desired_width(280.0));
                        if ui.button("Clear").clicked() {
                            self.search.clear();
                        }
                        if ui.small_button("×").clicked() {
                            self.show_search = false;
                            self.search.clear();
                        }
                    });
                }
                let query = self.search.to_lowercase();
                let mut rows: Vec<_> = self
                    .lib
                    .downloads
                    .iter()
                    .filter(|d| {
                        let filter = match self.filter.as_str() {
                            "All downloads" => true,
                            "Unfinished" => d.status != Status::Complete,
                            "Finished" => d.status == Status::Complete,
                            x if x.starts_with("queue:") => d.queue == x[6..],
                            x => d.category == x,
                        };
                        filter
                            && (query.is_empty()
                                || d.name().to_lowercase().contains(&query)
                                || d.url.to_lowercase().contains(&query)
                                || d.description.to_lowercase().contains(&query))
                    })
                    .collect();
                if let Some(column) = self.sort_column {
                    rows.sort_by(|left, right| {
                        let order = compare_downloads(column, left, right);
                        if self.sort_descending {
                            order.reverse()
                        } else {
                            order
                        }
                    });
                }
                visible_ids = rows.iter().map(|download| download.id).collect();
                if self.select_all_requested {
                    self.selected_ids = visible_ids.iter().copied().collect();
                    if !self
                        .selected
                        .is_some_and(|id| self.selected_ids.contains(&id))
                    {
                        self.selected = visible_ids.first().copied();
                    }
                    self.selection_anchor = self.selected;
                    self.select_all_requested = false;
                }
                let height = ui.available_height() - 8.0;
                egui::ScrollArea::horizontal()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(1180.0);
                        // Let clicks on text select the whole row and double-click open progress.
                        ui.style_mut().interaction.selectable_labels = false;
                        TableBuilder::new(ui)
                            .striped(false)
                            .resizable(true)
                            .sense(egui::Sense::click())
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .column(Column::initial(235.0).at_least(120.0))
                            .column(Column::initial(25.0).at_least(20.0))
                            .column(Column::initial(75.0).at_least(60.0))
                            .column(Column::initial(90.0).at_least(60.0))
                            .column(Column::initial(110.0).at_least(60.0))
                            .column(Column::initial(90.0).at_least(70.0))
                            .column(Column::initial(128.0).at_least(90.0))
                            .column(Column::initial(128.0).at_least(90.0))
                            .column(Column::initial(70.0).at_least(60.0))
                            .column(Column::remainder().at_least(180.0))
                            .min_scrolled_height(height.max(100.0))
                            .max_scroll_height(height.max(100.0))
                            .header(24.0, |mut header| {
                                for (column, label) in [
                                    (SortColumn::Filename, "File Name"),
                                    (SortColumn::Queue, "Q"),
                                    (SortColumn::Size, "Size"),
                                    (SortColumn::Status, "Status"),
                                    (SortColumn::Description, "Description"),
                                    (SortColumn::Speed, "Transfer rate"),
                                    (SortColumn::Date, "Last Try Date"),
                                    (SortColumn::Added, "Date Added"),
                                    (SortColumn::Remaining, "Time left"),
                                    (SortColumn::Folder, "Save To"),
                                ] {
                                    header.col(|ui| {
                                        ui.painter().rect_filled(
                                            ui.max_rect(),
                                            0.0,
                                            if self.lib.settings.dark {
                                                Color32::from_gray(40)
                                            } else {
                                                Color32::from_gray(245)
                                            },
                                        );
                                        let active = self.sort_column == Some(column);
                                        let title = if active {
                                            format!(
                                                "{label} {}",
                                                if self.sort_descending { "▼" } else { "▲" }
                                            )
                                        } else {
                                            label.to_string()
                                        };
                                        if ui
                                            .selectable_label(active, RichText::new(title))
                                            .on_hover_text("Click to sort; click again to reverse")
                                            .clicked()
                                        {
                                            if active {
                                                self.sort_descending = !self.sort_descending;
                                            } else {
                                                self.sort_column = Some(column);
                                                self.sort_descending = false;
                                            }
                                        }
                                    });
                                }
                            })
                            .body(|body| {
                                body.rows(21.0, rows.len(), |mut row| {
                                    let d = rows[row.index()];
                                    let id = d.id;
                                    row.set_selected(
                                        self.selected_ids.contains(&id)
                                            || self.selected == Some(id),
                                    );
                                    row.col(|ui| {
                                        let icon = self.art.sidebar(&d.category.to_lowercase());
                                        ui.image((icon.id(), Vec2::new(16.0, 13.0)));
                                        ui.add(egui::Label::new(d.name()).truncate())
                                            .on_hover_text(&d.url);
                                    });
                                    row.col(|ui| {
                                        if d.status != Status::Complete {
                                            let icon = self.art.sidebar("queues");
                                            ui.image((icon.id(), Vec2::new(16.0, 13.0)))
                                                .on_hover_text(&d.queue);
                                        }
                                    });
                                    row.col(|ui| {
                                        ui.label(
                                            d.total.map(human_bytes).unwrap_or_else(|| "—".into()),
                                        );
                                    });
                                    row.col(|ui| {
                                        ui.label(if d.status == Status::Downloading {
                                            format!("{:.2}%", d.percent() * 100.0)
                                        } else {
                                            d.status.label().into()
                                        });
                                    });
                                    row.col(|ui| {
                                        ui.add(egui::Label::new(&d.description).truncate())
                                            .on_hover_text(&d.description);
                                    });
                                    row.col(|ui| {
                                        ui.label(if d.speed > 0.0 {
                                            format!("{}/s", human_bytes(d.speed as u64))
                                        } else {
                                            "—".into()
                                        });
                                    });
                                    row.col(|ui| {
                                        ui.label(
                                            d.last_try
                                                .map(|t| {
                                                    t.with_timezone(&Local)
                                                        .format("%d %b %Y %H:%M")
                                                        .to_string()
                                                })
                                                .unwrap_or_else(|| "—".into()),
                                        );
                                    });
                                    row.col(|ui| {
                                        ui.label(
                                            d.added
                                                .with_timezone(&Local)
                                                .format("%d %b %Y %H:%M")
                                                .to_string(),
                                        );
                                    });
                                    row.col(|ui| {
                                        ui.label(if d.speed > 0.0 {
                                            human_time(
                                                d.total
                                                    .unwrap_or(d.downloaded)
                                                    .saturating_sub(d.downloaded)
                                                    as f64
                                                    / d.speed,
                                            )
                                        } else {
                                            "—".into()
                                        });
                                    });
                                    row.col(|ui| {
                                        ui.add(
                                            egui::Label::new(d.path.to_string_lossy()).truncate(),
                                        )
                                        .on_hover_text(d.path.to_string_lossy());
                                    });
                                    let response = row.response();
                                    if response.clicked() {
                                        let modifiers = ctx.input(|input| input.modifiers);
                                        row_selection = Some((
                                            id,
                                            modifiers.ctrl || modifiers.command,
                                            modifiers.shift,
                                            false,
                                        ));
                                    }
                                    if response.double_clicked() {
                                        row_action = Some((id, 7));
                                    }
                                    response.context_menu(|ui| {
                                        row_selection = Some((id, false, false, true));
                                        for (a, label) in [
                                            (0, "Resume selected"),
                                            (1, "Pause selected"),
                                            (2, "Open file"),
                                            (3, "Open folder"),
                                            (7, "Download progress"),
                                            (6, "Properties…"),
                                            (4, "Copy URL"),
                                            (5, "Remove selected from list"),
                                        ] {
                                            if ui.button(label).clicked() {
                                                row_action = Some((id, a));
                                                ui.close();
                                            }
                                        }
                                        ui.separator();
                                        ui.menu_button("Move this file to queue", |ui| {
                                            for (i, q) in
                                                self.lib.settings.queues.iter().enumerate()
                                            {
                                                if ui.button(&q.name).clicked() {
                                                    row_action = Some((id, 100 + i));
                                                    ui.close();
                                                }
                                            }
                                        });
                                    });
                                });
                            });
                    });
            });
        if let Some((id, control, shift, context_menu)) = row_selection {
            if context_menu && self.selected_ids.contains(&id) {
                self.selected = Some(id);
            } else {
                update_selection(
                    &mut self.selected_ids,
                    &mut self.selected,
                    &mut self.selection_anchor,
                    id,
                    &visible_ids,
                    control,
                    shift,
                );
            }
        }
        if let Some((id, a)) = row_action {
            match a {
                0 => self.resume_selected(),
                1 => self.pause_selected(),
                2 => {
                    if let Some(d) = self.lib.downloads.iter().find(|d| d.id == id) {
                        if d.status == Status::Complete {
                            open_path(&d.path);
                        }
                    }
                }
                3 => {
                    if let Some(d) = self.lib.downloads.iter().find(|d| d.id == id) {
                        if let Some(p) = d.path.parent() {
                            open_path(p);
                        }
                    }
                }
                4 => {
                    if let Some(d) = self.lib.downloads.iter().find(|d| d.id == id) {
                        ctx.copy_text(d.url.clone());
                    }
                }
                5 => self.request_delete(),
                6 => self.edit(id),
                7 => {
                    self.selected = Some(id);
                    self.transfer_open = true;
                }
                n if n >= 100 => {
                    if let Some(q) = self.lib.settings.queues.get(n - 100) {
                        if let Some(d) = self.lib.downloads.iter_mut().find(|d| d.id == id) {
                            d.queue = q.name.clone();
                        }
                    }
                }
                _ => (),
            }
        }
    }
    fn details(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{} downloads", self.lib.downloads.len()));
                ui.separator();
                ui.label(format!("{} active", self.controls.len()));
                ui.separator();
                let speed: f64 = self.lib.downloads.iter().map(|d| d.speed).sum();
                ui.label(format!("{}/s", human_bytes(speed as u64)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(if self.lib.settings.speed_limit_kib == 0 {
                        "Speed limit: Off".into()
                    } else {
                        format!(
                            "Per-download limit: {} KB/s",
                            self.lib.settings.speed_limit_kib
                        )
                    });
                });
            });
        });
        #[cfg(not(windows))]
        self.progress(ctx);
        #[cfg(windows)]
        self.sync_native_progress();
    }

    #[cfg(windows)]
    fn sync_native_progress(&mut self) {
        use native_progress::Action;
        for action in self.native_progress.poll() {
            match action {
                Action::Pause(id) => self.pause(id),
                Action::Resume(id) => self.start(id),
                Action::Cancel(id) => {
                    self.pause(id);
                    self.transfer_open = false;
                }
                Action::Closed => self.transfer_open = false,
                Action::Error(error) => {
                    self.message = Some(error);
                    self.transfer_open = false;
                }
                Action::Preferences {
                    id,
                    speed_limit_kib,
                    notify_on_complete,
                    open_folder_on_complete,
                    close_progress_on_complete,
                } => {
                    if let Some(download) = self.lib.downloads.iter_mut().find(|d| d.id == id) {
                        download.speed_limit_kib = speed_limit_kib;
                        download.notify_on_complete = notify_on_complete;
                        download.open_folder_on_complete = open_folder_on_complete;
                        download.close_progress_on_complete = close_progress_on_complete;
                        if let Some(control) = self.controls.get(&id) {
                            control.set_speed_limit(
                                speed_limit_kib.unwrap_or(self.lib.settings.speed_limit_kib),
                            );
                        }
                        self.save();
                    }
                }
            }
        }
        if self.transfer_open && !self.closing {
            if let Some(download) = self
                .selected
                .and_then(|id| self.lib.downloads.iter().find(|d| d.id == id))
            {
                self.native_progress
                    .show(download, self.lib.settings.speed_limit_kib);
            } else {
                self.transfer_open = false;
                self.native_progress.hide();
            }
        } else {
            self.native_progress.hide();
        }
    }
}
impl eframe::App for IdxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|input| input.viewport().close_requested()) && !self.closing {
            self.closing = true;
            self.pause_all();
            #[cfg(windows)]
            self.native_progress.hide();
        }
        self.tick(ctx);
        if self.closing {
            if self.controls.is_empty() {
                self.save();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                egui::CentralPanel::default().show(ctx, |ui| { ui.vertical_centered(|ui| { ui.add_space(80.0); ui.spinner(); ui.heading("Pausing downloads before closing…"); ui.label("Your partial downloads are being saved. A stalled connection may take up to 15 seconds to stop."); }); });
            }
            ctx.request_repaint_after(Duration::from_millis(100));
            return;
        }
        if ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::CTRL, egui::Key::N)
                || i.consume_key(egui::Modifiers::CTRL, egui::Key::B)
        }) {
            self.add_dialog(String::new());
        }
        if !ctx.wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
                self.request_delete();
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::A)) {
                self.select_all_requested = true;
            }
        }
        if !ctx.wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F))
        {
            self.show_search = true;
        }
        self.menu_toolbar(ctx);
        self.details(ctx);
        self.sidebar(ctx);
        self.list(ctx);
        self.dialogs(ctx);
        ctx.request_repaint_after(Duration::from_millis(180));
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.pause_all();
        for d in &mut self.lib.downloads {
            if d.status.active() {
                d.status = Status::Paused;
            }
        }
        self.save();
    }
}
#[cfg(not(windows))]
pub fn app_icon() -> egui::IconData {
    crate::ui_assets::app_icon()
}
fn compare_downloads(column: SortColumn, left: &Download, right: &Download) -> std::cmp::Ordering {
    let remaining = |download: &Download| {
        if download.speed > 0.0 {
            download
                .total
                .map(|total| total.saturating_sub(download.downloaded) as f64 / download.speed)
        } else {
            None
        }
    };
    let order = match column {
        SortColumn::Filename => left.name().to_lowercase().cmp(&right.name().to_lowercase()),
        SortColumn::Size => left.total.cmp(&right.total),
        SortColumn::Status => left.status.label().cmp(right.status.label()),
        SortColumn::Remaining => match (remaining(left), remaining(right)) {
            (Some(a), Some(b)) => a.total_cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        },
        SortColumn::Speed => left.speed.total_cmp(&right.speed),
        SortColumn::Date => left.last_try.cmp(&right.last_try),
        SortColumn::Queue => left.queue.cmp(&right.queue),
        SortColumn::Description => left
            .description
            .to_lowercase()
            .cmp(&right.description.to_lowercase()),
        SortColumn::Added => left.added.cmp(&right.added),
        SortColumn::Folder => left.path.cmp(&right.path),
        SortColumn::Referer => left
            .referer
            .to_lowercase()
            .cmp(&right.referer.to_lowercase()),
        SortColumn::ParentPage => left
            .parent_page
            .to_lowercase()
            .cmp(&right.parent_page.to_lowercase()),
    };
    order.then_with(|| left.id.cmp(&right.id))
}
fn update_selection(
    ids: &mut HashSet<u64>,
    primary: &mut Option<u64>,
    anchor: &mut Option<u64>,
    clicked: u64,
    visible: &[u64],
    control: bool,
    shift: bool,
) {
    if shift {
        let start = anchor
            .or(*primary)
            .and_then(|id| visible.iter().position(|item| *item == id));
        let end = visible.iter().position(|item| *item == clicked);
        if let (Some(start), Some(end)) = (start, end) {
            if !control {
                ids.clear();
            }
            ids.extend(visible[start.min(end)..=start.max(end)].iter().copied());
        } else {
            if !control {
                ids.clear();
            }
            ids.insert(clicked);
            *anchor = Some(clicked);
        }
        *primary = Some(clicked);
    } else if control {
        if !ids.insert(clicked) {
            ids.remove(&clicked);
        }
        *primary = if ids.contains(&clicked) {
            Some(clicked)
        } else {
            visible.iter().find(|id| ids.contains(id)).copied()
        };
        *anchor = Some(clicked);
    } else {
        ids.clear();
        ids.insert(clicked);
        *primary = Some(clicked);
        *anchor = Some(clicked);
    }
}
#[cfg(test)]
mod selection_tests {
    use super::*;
    static NEXT_FIXTURE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    fn category_test_app() -> (IdxApp, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "idx-category-controller-{}-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap(),
            NEXT_FIXTURE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut app = IdxApp::from_context(
            &egui::Context::default(),
            root.join("library.json"),
            TcpListener::bind("127.0.0.1:0").unwrap(),
        );
        app.lib.settings.download_dir = root.join("default");
        app.lib
            .settings
            .categories
            .push(crate::categories::Category {
                name: "Books".into(),
                extensions: "cbz".into(),
                sites_only: true,
                sites: "books.example.com".into(),
                folder: Some(root.join("books")),
                remember_last_path: true,
                ..Default::default()
            });
        (app, root)
    }

    #[test]
    fn grabber_sources_survive_add_and_restart_and_cancel_drops_stale_results() {
        let (mut app, root) = category_test_app();
        let first = "https://files.example/one.zip".to_string();
        let second = "https://files.example/two.zip".to_string();
        let parents = std::collections::BTreeMap::from([
            (first.clone(), "https://site.example/a.html".to_string()),
            (second.clone(), "https://site.example/b.html".to_string()),
        ]);
        let (tx, rx) = mpsc::channel();
        app.grab_rx = Some(rx);
        tx.send(crate::grabber::ScanEvent::Finished(Ok(
            crate::grabber::PageLinks {
                page_url: "https://site.example/index.html".into(),
                urls: vec![first.clone(), second.clone()],
                parents: parents.clone(),
                pages: 3,
                skipped: 0,
                limited: false,
                unknown_sizes: 0,
                duplicates: 0,
            },
        )))
        .unwrap();
        app.grab_url = "https://edited.example/".into();
        app.poll_grabber();
        app.add_dialog(format!("{first}\n{second}"));
        app.add.parent_page = app.grab_page.clone();
        app.add.parent_pages = app.grab_parents.clone();
        app.add.start = false;
        app.accept_add();
        assert!(app.add.error.is_empty());
        let restored = Library::load(&root.join("library.json")).unwrap();
        assert_eq!(restored.downloads.len(), 2);
        for download in restored.downloads {
            assert_eq!(download.parent_page, parents[&download.url]);
        }
        let (stale, rx) = mpsc::channel();
        app.grab_rx = Some(rx);
        app.cancel_grabber();
        assert!(
            stale
                .send(crate::grabber::ScanEvent::Finished(Err("stale".into())))
                .is_err()
        );
        app.poll_grabber();
        assert!(app.grab_error.is_empty());
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn batch_and_exact_add_use_category_rules_without_rewriting_manual_paths() {
        let (mut app, root) = category_test_app();
        app.add_dialog(
            "https://books.example.com/one.cbz\nhttps://other.example.com/two.cbz".into(),
        );
        app.add.start = false;
        app.accept_add();
        assert!(app.add.error.is_empty());
        assert_eq!(app.lib.downloads[0].category, "Books");
        assert_eq!(app.lib.downloads[0].path, root.join("books/one.cbz"));
        assert_eq!(app.lib.downloads[1].category, "Other");
        assert_eq!(app.lib.downloads[1].path, root.join("default/two.cbz"));
        app.add_dialog("https://books.example.com/three.cbz".into());
        app.add.start = false;
        app.add.exact_destination = true;
        app.add.remember_destination = true;
        app.add.folder = root.join("choice").to_string_lossy().into_owned();
        app.accept_add();
        assert_eq!(app.lib.downloads[2].path, root.join("choice/three.cbz"));
        assert_eq!(
            app.lib.settings.category_folder("Books"),
            root.join("choice")
        );
        let restored = Library::load(&root.join("library.json")).unwrap();
        assert_eq!(
            restored.settings.category_folder("Books"),
            root.join("choice")
        );
        assert_eq!(restored.downloads.len(), 3);
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn browser_category_destination_and_duplicate_ack_survive_restart() {
        let (mut app, root) = category_test_app();
        let url = "https://books.example.com/issue.cbz";
        app.accept_browser_download(
            url,
            "category-browser-test",
            "https://source.example/page",
            "",
            "",
        )
        .unwrap();
        app.accept_browser_download(
            url,
            "category-browser-test",
            "https://source.example/page",
            "",
            "",
        )
        .unwrap();
        let restored = Library::load(&root.join("library.json")).unwrap();
        assert_eq!(restored.downloads.len(), 1);
        assert_eq!(restored.downloads[0].category, "Books");
        assert_eq!(restored.downloads[0].path, root.join("books/issue.cbz"));
        assert_eq!(
            restored.downloads[0].parent_page,
            "https://source.example/page"
        );
        assert_eq!(restored.settings.download_dir, root.join("default"));
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn media_filename_and_referer_survive_the_browser_inbox() {
        let (mut app, root) = category_test_app();
        let url = "https://media.example/videoplayback?id=42";
        app.accept_browser_download(
            url,
            "media-1",
            "https://site.example/watch",
            "Film.mp4",
            "https://site.example/watch",
        )
        .unwrap();
        let saved = Library::load(&root.join("library.json")).unwrap();
        assert_eq!(saved.downloads[0].path.file_name().unwrap(), "Film.mp4");
        assert_eq!(saved.downloads[0].referer, "https://site.example/watch");
        assert_eq!(saved.downloads[0].parent_page, "https://site.example/watch");
        assert!(
            app.accept_browser_download(url, "media-2", "", "../escape.mp4", "")
                .is_err()
        );
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shift_selection_uses_display_order_and_control_toggles_one_row() {
        let mut ids = HashSet::new();
        let mut primary = None;
        let mut anchor = None;
        let visible = [4, 1, 3, 2];
        update_selection(
            &mut ids,
            &mut primary,
            &mut anchor,
            1,
            &visible,
            false,
            false,
        );
        update_selection(
            &mut ids,
            &mut primary,
            &mut anchor,
            2,
            &visible,
            false,
            true,
        );
        assert_eq!(ids, [1, 3, 2].into_iter().collect());
        assert_eq!(primary, Some(2));
        update_selection(
            &mut ids,
            &mut primary,
            &mut anchor,
            3,
            &visible,
            true,
            false,
        );
        assert_eq!(ids, [1, 2].into_iter().collect());
        update_selection(
            &mut ids,
            &mut primary,
            &mut anchor,
            4,
            &visible,
            true,
            false,
        );
        assert_eq!(ids, [1, 2, 4].into_iter().collect());
    }
    #[test]
    fn source_columns_sort_independently_and_keep_stable_ties() {
        let left = Download {
            id: 1,
            referer: "https://Z.example/".into(),
            parent_page: "https://a.example/".into(),
            ..Default::default()
        };
        let mut right = Download {
            id: 2,
            referer: "https://a.example/".into(),
            parent_page: "https://z.example/".into(),
            ..Default::default()
        };
        assert_eq!(
            compare_downloads(SortColumn::Referer, &left, &right),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_downloads(SortColumn::ParentPage, &left, &right),
            std::cmp::Ordering::Less
        );
        right.parent_page = "https://A.example/".into();
        assert_eq!(
            compare_downloads(SortColumn::ParentPage, &left, &right),
            std::cmp::Ordering::Less
        );
    }
    #[test]
    fn sort_compares_numeric_size_and_remaining_time() {
        let left = Download {
            id: 1,
            total: Some(1000),
            downloaded: 0,
            speed: 100.0,
            ..Default::default()
        };
        let right = Download {
            id: 2,
            total: Some(2000),
            downloaded: 1000,
            speed: 500.0,
            ..Default::default()
        };
        assert_eq!(
            compare_downloads(SortColumn::Size, &left, &right),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_downloads(SortColumn::Remaining, &left, &right),
            std::cmp::Ordering::Greater
        );
    }
}
fn open_path(path: &Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer.exe").arg(path).spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}
