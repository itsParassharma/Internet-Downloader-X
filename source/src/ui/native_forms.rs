//! Modal Windows forms backed by the original extracted dialog templates.
//! Resource geometry and fonts are kept in the resource pack. The handlers here
//! connect those controls to IDX's downloader; no original executable code runs.
#![allow(unsafe_op_in_unsafe_fn)]

use super::{AddForm, IdxApp};
use crate::model::{Download, Status, filename_from_url};
use std::{
    cell::RefCell,
    collections::HashSet,
    ffi::c_void,
    path::PathBuf,
    ptr::{null, null_mut},
};

pub(super) type Handle = *mut c_void;
type DialogProc = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;
const WM_INITDIALOG: u32 = 0x0110;
const WM_COMMAND: u32 = 0x0111;
const WM_CLOSE: u32 = 0x0010;
const WM_TIMER: u32 = 0x0113;
const WM_DESTROY: u32 = 0x0002;
const GWLP_USERDATA: i32 = -21;
const TIMER: usize = 711;

#[link(name = "user32")]
unsafe extern "system" {
    fn DialogBoxIndirectParamW(
        instance: Handle,
        template: *const c_void,
        owner: Handle,
        callback: Option<DialogProc>,
        param: isize,
    ) -> isize;
    fn EndDialog(window: Handle, result: isize) -> i32;
    fn GetDlgItem(window: Handle, id: i32) -> Handle;
    fn SetDlgItemTextW(window: Handle, id: i32, text: *const u16) -> i32;
    fn GetWindowTextLengthW(window: Handle) -> i32;
    fn GetWindowTextW(window: Handle, text: *mut u16, length: i32) -> i32;
    fn SetWindowTextW(window: Handle, text: *const u16) -> i32;
    fn SendMessageW(window: Handle, message: u32, wparam: usize, lparam: isize) -> isize;
    fn SetWindowLongPtrW(window: Handle, index: i32, value: isize) -> isize;
    fn GetWindowLongPtrW(window: Handle, index: i32) -> isize;
    fn EnableWindow(window: Handle, enabled: i32) -> i32;
    fn ShowWindow(window: Handle, show: i32) -> i32;
    fn LoadIconW(instance: Handle, name: *const u16) -> Handle;
    fn SetTimer(window: Handle, id: usize, interval: u32, callback: *const c_void) -> usize;
    fn KillTimer(window: Handle, id: usize) -> i32;
    fn MessageBoxW(owner: Handle, text: *const u16, title: *const u16, flags: u32) -> i32;
    fn EnumChildWindows(
        window: Handle,
        callback: Option<unsafe extern "system" fn(Handle, isize) -> i32>,
        param: isize,
    ) -> i32;
    fn DestroyIcon(icon: Handle) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> Handle;
}
#[link(name = "shell32")]
unsafe extern "system" {
    fn SHGetFileInfoW(
        path: *const u16,
        attributes: u32,
        info: *mut ShellFileInfo,
        size: u32,
        flags: u32,
    ) -> usize;
    fn SHOpenWithDialog(owner: Handle, info: *const OpenAsInfo) -> i32;
}

#[repr(C)]
struct ShellFileInfo {
    icon: Handle,
    icon_index: i32,
    attributes: u32,
    display_name: [u16; 260],
    type_name: [u16; 80],
}
#[repr(C)]
struct OpenAsInfo {
    file: *const u16,
    class: *const u16,
    flags: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Address,
    Info,
    Properties,
    Find,
    About,
    Delete,
    Batch,
    SpeedLimit,
    Complete,
    Advanced,
}
struct State {
    app: *mut IdxApp,
    context: eframe::egui::Context,
    kind: Kind,
    form: AddForm,
    download: Option<Download>,
    count: usize,
    accepted: bool,
    multiple: bool,
    auto_path: bool,
    file_icon: Handle,
}
impl State {
    fn new(kind: Kind, app: *mut IdxApp) -> Self {
        Self {
            app,
            context: eframe::egui::Context::default(),
            kind,
            form: AddForm::default(),
            download: None,
            count: 0,
            accepted: false,
            multiple: false,
            auto_path: true,
            file_icon: null_mut(),
        }
    }
}

// DialogBox dispatches parent timers and synchronous control notifications. The
// main host guards its own model access; this guard also stops nested callbacks
// from creating a second mutable reference while one handler is active.
thread_local! { static ACTIVE_HANDLERS: RefCell<HashSet<usize>> = RefCell::new(HashSet::new()); }
struct CallbackGuard(usize);
impl Drop for CallbackGuard {
    fn drop(&mut self) {
        ACTIVE_HANDLERS.with(|active| {
            active.borrow_mut().remove(&self.0);
        });
    }
}

pub(super) fn add(owner: Handle, app: &mut IdxApp) {
    let mut state = State::new(Kind::Address, app);
    state.form = std::mem::take(&mut app.add);
    state.form.open = true;
    if state.form.folder.is_empty() {
        state.form.folder = app.lib.settings.download_dir.to_string_lossy().into_owned();
    }
    if state.form.edit_id.is_some() {
        app.add = state.form;
        properties(owner, app);
        return;
    }
    let has_urls = !state.form.urls.trim().is_empty();
    if !has_urls && !run(owner, 148, &mut state) {
        return;
    }
    state.kind = Kind::Info;
    state.accepted = false;
    state.multiple = crate::batch::expand_urls(&state.form.urls)
        .map(|urls| urls.len() > 1)
        .unwrap_or(false);
    run(owner, 130, &mut state);
}

pub(super) fn batch(owner: Handle, app: &mut IdxApp) {
    let mut state = State::new(Kind::Batch, app);
    state.form = std::mem::take(&mut app.add);
    if state.form.folder.is_empty() {
        state.form.folder = app.lib.settings.download_dir.to_string_lossy().into_owned();
    }
    state.form.open = true;
    if !run(owner, 280, &mut state) {
        return;
    }
    state.kind = Kind::Info;
    state.multiple = true;
    state.accepted = false;
    run(owner, 130, &mut state);
}

pub(super) fn properties(owner: Handle, app: &mut IdxApp) {
    let Some(id) = app.add.edit_id.or(app.selected) else {
        return;
    };
    let Some(download) = app.lib.downloads.iter().find(|d| d.id == id).cloned() else {
        return;
    };
    if app.add.edit_id == Some(id) {
        app.add = AddForm::default();
    }
    let mut state = State::new(Kind::Properties, app);
    state.form = AddForm {
        open: true,
        urls: download.url.clone(),
        filename: download.name(),
        folder: download
            .path
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        queue: app
            .lib
            .settings
            .queues
            .iter()
            .position(|q| q.name == download.queue)
            .unwrap_or(0),
        category: app
            .lib
            .settings
            .categories
            .iter()
            .position(|c| c.name == download.category)
            .map(|i| i + 1)
            .unwrap_or(0),
        start: false,
        advanced: true,
        username: download.username.clone(),
        password: download.password.clone(),
        referer: download.referer.clone(),
        sha256: download.sha256.clone(),
        description: download.description.clone(),
        edit_id: Some(id),
        ..Default::default()
    };
    state.download = Some(download);
    run(owner, 135, &mut state);
}

pub(super) fn find(owner: Handle, app: &mut IdxApp) {
    let mut state = State::new(Kind::Find, app);
    state.form.urls = app.search.clone();
    run(owner, 506, &mut state);
}

pub(super) fn about(owner: Handle) {
    run(owner, 100, &mut State::new(Kind::About, null_mut()));
}
pub(super) fn delete(owner: Handle, count: usize) -> bool {
    let mut state = State::new(Kind::Delete, null_mut());
    state.count = count;
    run(owner, 338, &mut state)
}
pub(super) fn speed_limit(owner: Handle, app: &mut IdxApp) {
    run(owner, 337, &mut State::new(Kind::SpeedLimit, app));
}
pub(super) fn complete(owner: Handle, app: &mut IdxApp, id: u64) {
    let Some(download) = app
        .lib
        .downloads
        .iter()
        .find(|d| d.id == id && d.status == Status::Complete)
        .cloned()
    else {
        return;
    };
    let mut state = State::new(Kind::Complete, app);
    state.download = Some(download);
    run(owner, 286, &mut state);
}

fn run(owner: Handle, resource: u32, state: &mut State) -> bool {
    let template = if resource == 0 {
        advanced_template()
    } else {
        super::native_resources::dialog(resource)
    };
    if template.is_empty() {
        return false;
    }
    let result = unsafe {
        DialogBoxIndirectParamW(
            GetModuleHandleW(null()),
            template.as_ptr().cast(),
            owner,
            Some(dialog_proc),
            state as *mut State as isize,
        )
    };
    if result == -1 {
        let error = std::io::Error::last_os_error();
        unsafe {
            notice(
                owner,
                &format!("The native dialog could not be opened: {error}"),
            );
        }
    }
    if !state.file_icon.is_null() {
        unsafe {
            DestroyIcon(state.file_icon);
        }
        state.file_icon = null_mut();
    }
    state.accepted
}

unsafe extern "system" fn dialog_proc(
    window: Handle,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if !ACTIVE_HANDLERS.with(|active| active.borrow_mut().insert(window as usize)) {
        return 0;
    }
    let _guard = CallbackGuard(window as usize);
    let pointer = if message == WM_INITDIALOG {
        SetWindowLongPtrW(window, GWLP_USERDATA, lparam);
        lparam as *mut State
    } else {
        GetWindowLongPtrW(window, GWLP_USERDATA) as *mut State
    };
    if pointer.is_null() {
        return 0;
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handle(window, message, wparam, &mut *pointer)
    }));
    match result {
        Ok(result) => result,
        Err(_) => {
            EndDialog(window, 0);
            1
        }
    }
}

unsafe fn handle(window: Handle, message: u32, wparam: usize, state: &mut State) -> isize {
    match message {
        WM_INITDIALOG => {
            let icon = LoadIconW(GetModuleHandleW(null()), 101usize as *const u16);
            SendMessageW(window, 0x0080, 0, icon as isize);
            SendMessageW(window, 0x0080, 1, icon as isize);
            if !state.app.is_null() {
                SetTimer(window, TIMER, 200, null());
            }
            init(window, state);
            1
        }
        WM_TIMER if wparam == TIMER && !state.app.is_null() => {
            // The borrow ends before any subsequent modal interaction.
            {
                let app = &mut *state.app;
                app.tick(&state.context);
                app.sync_native_progress();
            }
            if state.kind == Kind::Properties {
                refresh_properties(window, state);
            }
            1
        }
        WM_CLOSE => {
            EndDialog(window, 0);
            1
        }
        WM_DESTROY => {
            KillTimer(window, TIMER);
            1
        }
        WM_COMMAND => {
            let id = (wparam & 0xffff) as i32;
            let notification = (wparam >> 16) as u16;
            if id == 2 || id == 1080 || id == 1079 || (state.kind == Kind::Delete && id == 1202) {
                EndDialog(window, 0);
                return 1;
            }
            command(window, id, notification, state)
        }
        _ => 0,
    }
}

unsafe fn init(window: Handle, state: &mut State) {
    match state.kind {
        Kind::Address | Kind::Batch => {
            text(window, 1007, &state.form.urls);
            text(window, 1033, &state.form.username);
            text(window, 1041, &state.form.password);
            check(window, 1182, !state.form.username.is_empty());
            auth_enabled(window);
            if !state.app.is_null() {
                for download in (&*state.app).lib.downloads.iter().rev().take(30) {
                    combo_add(window, 1007, &download.url);
                }
                text(window, 1007, &state.form.urls);
            }
            if state.kind == Kind::Batch {
                text(
                    window,
                    1553,
                    "Add a sequence of file addresses to the download queue. Use * for a numeric or letter sequence.\r\nFor example: https://example.com/pictures/img*.jpg",
                );
                text(window, 1542, "1");
                text(window, 1040, "10");
                text(window, 1070, "3");
                text(window, 1546, "a");
                text(window, 1210, "z");
                check(window, 1540, true);
                check(window, 1541, false);
                SendMessageW(GetDlgItem(window, 1543), 0x046f, 0, 999999);
                SendMessageW(GetDlgItem(window, 1544), 0x046f, 0, 999999);
                SendMessageW(GetDlgItem(window, 1545), 0x046f, 1, 12);
                batch_mode(window);
                batch_preview(window);
            }
            SendMessageW(GetDlgItem(window, 1007), 0x0142, 16384, 0);
        }
        Kind::Info => {
            SetWindowTextW(window, wide("Download File Info").as_ptr());
            let display = if state.multiple {
                crate::batch::expand_urls(&state.form.urls)
                    .map(|urls| {
                        format!(
                            "{} addresses — {}",
                            urls.len(),
                            urls.first().cloned().unwrap_or_default()
                        )
                    })
                    .unwrap_or_else(|_| state.form.urls.clone())
            } else {
                state.form.urls.clone()
            };
            text(window, 1007, &display);
            combo_add(window, 1209, "Automatic");
            for category in &(&*state.app).lib.settings.categories {
                combo_add(window, 1209, &category.name);
            }
            combo_select(window, 1209, state.form.category);
            text(window, 1009, &state.form.description);
            text(window, 1609, "Remember this path for this category");
            text(
                window,
                1101,
                if state.multiple {
                    "Multiple files"
                } else {
                    "Size unknown"
                },
            );
            enable(window, 1697, true);
            text(window, 1173, "More...");
            show(window, 1173, true);
            enable(window, 1173, true);
            update_save_path(window, state);
            let file_name = if state.form.filename.is_empty() {
                crate::batch::expand_urls(&state.form.urls)
                    .ok()
                    .and_then(|urls| urls.into_iter().next())
                    .and_then(|url| filename_from_url(&url).ok())
                    .unwrap_or_else(|| "download.bin".into())
            } else {
                state.form.filename.clone()
            };
            set_file_icon(window, 1094, state, &file_name);
        }
        Kind::Properties => {
            if let Some(download) = &state.download {
                text(window, 1081, &download.name());
                text(window, 1093, &download.category);
                text(window, 1096, &download.path.to_string_lossy());
                text(window, 1097, &download.url);
                text(window, 1099, &download.description);
                text(window, 1569, &download.referer);
                text(
                    window,
                    1734,
                    if download.parent_page.is_empty() {
                        "Unknown"
                    } else {
                        &download.parent_page
                    },
                );
                text(window, 1033, &download.username);
                text(window, 1041, &download.password);
                let editable = !download.status.active()
                    && !matches!(download.status, Status::Complete | Status::Verifying);
                for id in [1096, 1082, 1097, 1569, 1033, 1041] {
                    enable(window, id, editable);
                }
                enable(
                    window,
                    1099,
                    !download.status.active() && download.status != Status::Verifying,
                );
                text(
                    window,
                    1164,
                    if download.status == Status::Complete {
                        "Open"
                    } else {
                        "More..."
                    },
                );
                enable(
                    window,
                    1164,
                    editable || (download.status == Status::Complete && download.path.is_file()),
                );
                let name = download.name();
                set_file_icon(window, 1094, state, &name);
            }
            refresh_properties(window, state);
        }
        Kind::Find => {
            text(window, 1808, &state.form.urls);
            for id in [1861, 1858, 1859] {
                check(window, id, true);
            }
        }
        Kind::About => {
            text(window, 1180, "IDX – Internet Downloader X");
            text(window, 1262, concat!("Version ", env!("CARGO_PKG_VERSION")));
            for id in [
                1161, 1286, 1181, 1085, 1185, 1814, 1818, 1284, 1187, 1285, 1189,
            ] {
                show(window, id, false);
            }
            text(
                window,
                1283,
                "IDX is a download manager built in Rust.\r\n\r\nParallel downloads, pause and resume, categories, queues, scheduling and browser integration.\r\n\r\nRust source: MIT license. See the bundled license and resource notices.",
            );
            // The About template has two anonymous statics; replace its logo and
            // keep the application identity consistent with the rest of IDX.
            EnumChildWindows(window, Some(about_child), 0);
        }
        Kind::Delete => {
            text(
                window,
                1715,
                &format!(
                    "Remove {} selected download(s) from the IDX list?",
                    state.count
                ),
            );
            text(window, 1713, "Downloaded files are kept on your disk.");
            check(window, 1713, false);
            enable(window, 1713, false);
            show(window, 1654, false); // No preference mutation through a bool-only API.
        }
        Kind::SpeedLimit => {
            let limit = (&*state.app).lib.settings.speed_limit_kib;
            text(
                window,
                1660,
                &if limit == 0 { 512 } else { limit }.to_string(),
            );
            text(
                window,
                1704,
                "Enable Speed Limiter (saved for the next IDX launch)",
            );
            check(window, 1704, limit > 0);
            enable(window, 1660, limit > 0);
        }
        Kind::Complete => {
            if let Some(download) = &state.download {
                text(
                    window,
                    1301,
                    &format!(
                        "Downloaded {} bytes.\r\n{}",
                        download.downloaded,
                        download.name()
                    ),
                );
                text(window, 1603, &download.url);
                text(window, 1040, &download.path.to_string_lossy());
                enable(window, 1164, download.path.is_file());
                enable(window, 1639, download.path.is_file());
            }
            show(window, 1607, false); // Global notification suppression is not implemented.
            show(window, 1023, false); // Original dynamic file-type selector is not used.
        }
        Kind::Advanced => {
            for queue in &(&*state.app).lib.settings.queues {
                combo_add(window, 2001, &queue.name);
            }
            combo_select(window, 2001, state.form.queue);
            text(window, 2002, &state.form.referer);
            text(window, 2003, &state.form.sha256);
            text(window, 2004, &state.form.username);
            text(window, 2005, &state.form.password);
        }
    }
}

unsafe extern "system" fn about_child(window: Handle, _: isize) -> i32 {
    let value = window_text(window);
    if value.contains("Tonec") || value.starts_with("Original interface resources") {
        SetWindowTextW(
            window,
            wide("IDX Rust source © 2026 IDX Contributors.").as_ptr(),
        );
    }
    if GetWindowLongPtrW(window, -16) as u32 & 0x1f == 3 {
        let icon = LoadIconW(GetModuleHandleW(null()), 101usize as *const u16);
        SendMessageW(window, 0x0170, icon as usize, 0);
    }
    1
}

unsafe fn command(window: Handle, id: i32, notification: u16, state: &mut State) -> isize {
    match state.kind {
        Kind::Address => match id {
            1182 => {
                auth_enabled(window);
                1
            }
            1022 | 1 => {
                state.form.urls = get(window, 1007).trim().into();
                if let Err(error) = crate::batch::expand_urls(&state.form.urls) {
                    notice(window, &error);
                    return 1;
                }
                read_auth(window, &mut state.form);
                state.accepted = true;
                EndDialog(window, 1);
                1
            }
            _ => 0,
        },
        Kind::Batch => match id {
            1182 => {
                auth_enabled(window);
                1
            }
            1540 | 1541 => {
                check(window, 1540, id == 1540);
                check(window, 1541, id == 1541);
                batch_mode(window);
                batch_preview(window);
                1
            }
            1007 | 1542 | 1040 | 1070 | 1546 | 1210
                if notification == 0x0300 || notification == 5 || notification == 1 =>
            {
                batch_preview(window);
                1
            }
            1 => {
                match batch_urls(window) {
                    Ok(urls) => {
                        state.form.urls = urls.join("\n");
                        read_auth(window, &mut state.form);
                        state.accepted = true;
                        EndDialog(window, 1);
                    }
                    Err(error) => notice(window, &error),
                }
                1
            }
            _ => 0,
        },
        Kind::Info => match id {
            1697 => {
                let app = &*state.app;
                let categories = app.lib.settings.categories.clone();
                let initial = crate::categories::Category {
                    folder: Some(app.lib.settings.download_dir.clone()),
                    ..Default::default()
                };
                match super::native_settings::edit_category(
                    window,
                    state.app,
                    initial,
                    &categories,
                    None,
                ) {
                    Ok(Some(category)) => {
                        let app = &mut *state.app;
                        app.lib.settings.categories.push(category.clone());
                        state.form.category = app.lib.settings.categories.len();
                        combo_add(window, 1209, &category.name);
                        combo_select(window, 1209, state.form.category);
                        app.save();
                        if state.auto_path {
                            update_save_path(window, state);
                        }
                    }
                    Err(error) => notice(window, &error),
                    _ => {}
                }
                1
            }
            1173 => {
                more(window, state);
                1
            }
            1209 if notification == 1 => {
                state.form.category = combo_index(window, 1209);
                if state.auto_path {
                    update_save_path(window, state);
                }
                1
            }
            1008 if notification == 5 => {
                state.auto_path = false;
                1
            }
            1608 => {
                let path = PathBuf::from(get(window, 1008));
                let selected = if state.multiple {
                    rfd::FileDialog::new().set_directory(&path).pick_folder()
                } else {
                    let name = path
                        .file_name()
                        .map(|v| v.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    rfd::FileDialog::new()
                        .set_directory(path.parent().unwrap_or(&path))
                        .set_file_name(name)
                        .save_file()
                };
                if let Some(path) = selected {
                    text(window, 1008, &path.to_string_lossy());
                    state.auto_path = false;
                }
                1
            }
            1006 | 1004 | 1 => {
                state.form.category = combo_index(window, 1209);
                state.form.description = get(window, 1009);
                state.form.start = id != 1004;
                let destination = PathBuf::from(get(window, 1008).trim());
                if state.multiple {
                    state.form.folder = destination.to_string_lossy().into_owned();
                    state.form.filename.clear();
                } else if let (Some(parent), Some(name)) =
                    (destination.parent(), destination.file_name())
                {
                    state.form.folder = parent.to_string_lossy().into_owned();
                    state.form.filename = name.to_string_lossy().into_owned();
                } else {
                    notice(window, "Choose an absolute destination and a filename.");
                    return 1;
                }
                if accept_form(state, !state.multiple || !state.auto_path) {
                    if checked(window, 1609) {
                        let app = &mut *state.app;
                        let folders: Vec<_> = app
                            .lib
                            .downloads
                            .iter()
                            .filter(|d| app.selected_ids.contains(&d.id))
                            .filter_map(|d| {
                                d.path.parent().map(|p| (d.category.clone(), p.to_owned()))
                            })
                            .collect();
                        for (category, folder) in folders {
                            app.lib
                                .settings
                                .remember_category_folder(&category, &folder, true);
                        }
                        app.save();
                    }
                    state.accepted = true;
                    EndDialog(window, 1);
                } else {
                    notice(window, &state.form.error);
                }
                1
            }
            _ => 0,
        },
        Kind::Properties => match id {
            1082 => {
                let path = PathBuf::from(get(window, 1096));
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(path.parent().unwrap_or(&path))
                    .set_file_name(path.file_name().unwrap_or_default().to_string_lossy())
                    .save_file()
                {
                    text(window, 1096, &path.to_string_lossy());
                }
                1
            }
            1164 => {
                if let Some(download) = &state.download {
                    if download.status == Status::Complete {
                        if download.path.is_file() {
                            super::open_path(&download.path);
                        }
                    } else {
                        state.form.referer = get(window, 1569);
                        state.form.username = get(window, 1033);
                        state.form.password = get(window, 1041);
                        more(window, state);
                        text(window, 1569, &state.form.referer);
                        text(window, 1033, &state.form.username);
                        text(window, 1041, &state.form.password);
                    }
                }
                1
            }
            1092 | 1 => {
                let Some(id) = state.download.as_ref().map(|d| d.id) else {
                    return 1;
                };
                let current = (&*state.app)
                    .lib
                    .downloads
                    .iter()
                    .find(|d| d.id == id)
                    .map(|d| d.status);
                if matches!(current, Some(Status::Complete)) {
                    {
                        let app = &mut *state.app;
                        if let Some(download) = app.lib.downloads.iter_mut().find(|d| d.id == id) {
                            download.description = get(window, 1099);
                        }
                        app.save();
                    }
                    state.accepted = true;
                    EndDialog(window, 1);
                    return 1;
                }
                if current.is_none()
                    || current.is_some_and(|s| s.active() || s == Status::Verifying)
                {
                    EndDialog(window, 0);
                    return 1;
                }
                let path = PathBuf::from(get(window, 1096).trim());
                let (Some(parent), Some(filename)) = (path.parent(), path.file_name()) else {
                    notice(window, "Choose an absolute destination and a filename.");
                    return 1;
                };
                state.form.folder = parent.to_string_lossy().into_owned();
                state.form.filename = filename.to_string_lossy().into_owned();
                state.form.urls = get(window, 1097);
                state.form.description = get(window, 1099);
                state.form.referer = get(window, 1569);
                state.form.username = get(window, 1033);
                state.form.password = get(window, 1041);
                state.form.start = false;
                if accept_form(state, true) {
                    state.accepted = true;
                    EndDialog(window, 1);
                } else {
                    notice(window, &state.form.error);
                }
                1
            }
            _ => 0,
        },
        Kind::Find => match id {
            1673 | 1 => {
                let query = get(window, 1808);
                if query.is_empty() {
                    notice(window, "Enter a filename, description, or address to find.");
                    return 1;
                }
                let fields = [
                    checked(window, 1861),
                    checked(window, 1858),
                    checked(window, 1859),
                ];
                if !fields.iter().any(|checked| *checked) {
                    notice(window, "Choose at least one field to search.");
                    return 1;
                }
                let case = checked(window, 1811);
                let exact = checked(window, 1812);
                let query = if case { query } else { query.to_lowercase() };
                let matches = |value: &str| {
                    let value = if case {
                        value.to_owned()
                    } else {
                        value.to_lowercase()
                    };
                    if exact {
                        value == query
                    } else {
                        value.contains(&query)
                    }
                };
                let ids: Vec<_> = (&*state.app)
                    .lib
                    .downloads
                    .iter()
                    .filter(|d| {
                        (fields[0] && matches(&d.name()))
                            || (fields[1] && matches(&d.description))
                            || (fields[2]
                                && (matches(&d.url)
                                    || matches(&d.parent_page)
                                    || matches(&d.referer)))
                    })
                    .map(|d| d.id)
                    .collect();
                if ids.is_empty() {
                    notice(window, "No matching downloads were found.");
                    return 1;
                }
                {
                    let app = &mut *state.app;
                    app.filter = "All downloads".into();
                    app.search.clear();
                    app.selected = ids.first().copied();
                    app.selected_ids = ids.into_iter().collect();
                    app.selection_anchor = app.selected;
                }
                state.accepted = true;
                EndDialog(window, 1);
                1
            }
            _ => 0,
        },
        Kind::Delete if id == 1163 || id == 1 => {
            state.accepted = true;
            EndDialog(window, 1);
            1
        }
        Kind::About if id == 1 => {
            EndDialog(window, 1);
            1
        }
        Kind::SpeedLimit => match id {
            1704 => {
                enable(window, 1660, checked(window, 1704));
                1
            }
            1 => {
                let limit = if checked(window, 1704) {
                    match get(window, 1660).parse::<u64>() {
                        Ok(value @ 1..=1_024_000) => value,
                        _ => {
                            notice(window, "Enter a limit between 1 and 1,024,000 KB/s.");
                            return 1;
                        }
                    }
                } else {
                    0
                };
                {
                    let app = &mut *state.app;
                    app.lib.settings.speed_limit_kib = limit;
                    for (id, control) in &app.controls {
                        if app
                            .lib
                            .downloads
                            .iter()
                            .find(|d| d.id == *id)
                            .is_some_and(|d| d.speed_limit_kib.is_none())
                        {
                            control.set_speed_limit(limit);
                        }
                    }
                    app.save();
                }
                state.accepted = true;
                EndDialog(window, 1);
                1
            }
            _ => 0,
        },
        Kind::Complete => match id {
            1164 | 1606 | 1639 => {
                if let Some(download) = &state.download {
                    if id == 1606 {
                        if let Some(folder) = download.path.parent() {
                            super::open_path(folder);
                        }
                    } else if id == 1639 {
                        let path = wide(&download.path.to_string_lossy());
                        let info = OpenAsInfo {
                            file: path.as_ptr(),
                            class: null(),
                            flags: 4,
                        };
                        SHOpenWithDialog(window, &info);
                    } else if download.path.is_file() {
                        super::open_path(&download.path);
                    }
                }
                1
            }
            1 => {
                EndDialog(window, 1);
                1
            }
            _ => 0,
        },
        Kind::Advanced if id == 1 => {
            let sha = get(window, 2003).trim().to_owned();
            if !sha.is_empty() && (sha.len() != 64 || !sha.bytes().all(|c| c.is_ascii_hexdigit())) {
                notice(
                    window,
                    "SHA-256 must be blank or contain 64 hexadecimal characters.",
                );
                return 1;
            }
            state.form.queue = combo_index(window, 2001);
            state.form.referer = get(window, 2002);
            state.form.sha256 = sha;
            state.form.username = get(window, 2004);
            state.form.password = get(window, 2005);
            state.accepted = true;
            EndDialog(window, 1);
            1
        }
        _ => 0,
    }
}

unsafe fn accept_form(state: &mut State, exact_destination: bool) -> bool {
    let app = &mut *state.app;
    let pending = std::mem::replace(&mut app.add, std::mem::take(&mut state.form));
    app.add.open = true;
    app.add.exact_destination = exact_destination;
    app.add.remember_destination = !state.auto_path && app.add.edit_id.is_none();
    app.accept_add();
    let accepted = !app.add.open;
    state.form = std::mem::replace(&mut app.add, pending);
    accepted
}

unsafe fn update_save_path(window: Handle, state: &mut State) {
    let app = &*state.app;
    let first = crate::batch::expand_urls(&state.form.urls)
        .ok()
        .and_then(|u| u.into_iter().next())
        .unwrap_or_default();
    let name = if state.form.filename.is_empty() {
        filename_from_url(&first).unwrap_or_else(|_| "download.bin".into())
    } else {
        state.form.filename.clone()
    };
    let category = app
        .lib
        .settings
        .category_for(&name, &first, state.form.category);
    let requested = PathBuf::from(&state.form.folder);
    let folder =
        app.lib
            .settings
            .destination_folder(&category, &requested, state.form.edit_id.is_some());
    let selected = if state.multiple {
        requested
    } else {
        folder.join(&name)
    };
    text(window, 1008, &selected.to_string_lossy());
    text(window, 1815, &folder.to_string_lossy());
    SendMessageW(GetDlgItem(window, 1815), 0x00cf, 1, 0);
}

unsafe fn refresh_properties(window: Handle, state: &mut State) {
    let Some(id) = state.download.as_ref().map(|d| d.id) else {
        return;
    };
    let Some(download) = (&*state.app)
        .lib
        .downloads
        .iter()
        .find(|d| d.id == id)
        .cloned()
    else {
        return;
    };
    text(window, 1098, download.status.label());
    text(
        window,
        1095,
        &download
            .total
            .map(|size| format!("{size} bytes"))
            .unwrap_or_else(|| "Unknown".into()),
    );
    enable(
        window,
        1164,
        (!download.status.active()
            && download.status != Status::Verifying
            && download.status != Status::Complete)
            || (download.status == Status::Complete && download.path.is_file()),
    );
    text(
        window,
        1164,
        if download.status == Status::Complete {
            "Open"
        } else {
            "More..."
        },
    );
    let editable = !download.status.active()
        && !matches!(download.status, Status::Complete | Status::Verifying);
    for control in [1096, 1082, 1097, 1569, 1033, 1041] {
        enable(window, control, editable);
    }
    enable(
        window,
        1099,
        !download.status.active() && download.status != Status::Verifying,
    );
    state.download = Some(download);
}

unsafe fn more(window: Handle, state: &mut State) {
    let mut extra = State::new(Kind::Advanced, state.app);
    extra.form = std::mem::take(&mut state.form);
    run(window, 0, &mut extra);
    state.form = extra.form;
}

// IDX has checksum and queue inputs that have no counterpart on the original
// File Info resource. This supplementary native form keeps them available
// without changing any of that resource's original coordinates.
fn advanced_template() -> Vec<u32> {
    fn word(out: &mut Vec<u8>, value: u16) {
        out.extend_from_slice(&value.to_le_bytes());
    }
    fn dword(out: &mut Vec<u8>, value: u32) {
        out.extend_from_slice(&value.to_le_bytes());
    }
    fn string(out: &mut Vec<u8>, value: &str) {
        for code in value.encode_utf16().chain(Some(0)) {
            word(out, code);
        }
    }
    fn item(out: &mut Vec<u8>, id: u16, class: u16, caption: &str, rect: [u16; 4], style: u32) {
        while out.len() % 4 != 0 {
            out.push(0);
        }
        dword(out, style);
        dword(out, 0);
        for coordinate in rect {
            word(out, coordinate);
        }
        word(out, id);
        word(out, 0xffff);
        word(out, class);
        string(out, caption);
        word(out, 0);
    }
    let mut out = Vec::new();
    dword(&mut out, 0x80c800c0);
    dword(&mut out, 0);
    word(&mut out, 13);
    for value in [0, 0, 290, 164] {
        word(&mut out, value);
    }
    word(&mut out, 0);
    word(&mut out, 0);
    string(&mut out, "Additional download options");
    word(&mut out, 8);
    string(&mut out, "Tahoma");
    for (id, label, y) in [
        (2001, "Queue", 7),
        (2002, "Referer", 31),
        (2003, "SHA-256", 55),
        (2004, "User name", 79),
        (2005, "Password", 103),
    ] {
        item(&mut out, 0xffff, 0x82, label, [7, y, 54, 10], 0x50020000);
        item(
            &mut out,
            id,
            if id == 2001 { 0x85 } else { 0x81 },
            "",
            [65, y - 2, 215, if id == 2001 { 100 } else { 14 }],
            if id == 2001 {
                0x50210003
            } else if id == 2005 {
                0x508100a0
            } else {
                0x50810080
            },
        );
    }
    item(
        &mut out,
        0xffff,
        0x82,
        "Website credentials are kept in memory for this session only.",
        [7, 122, 273, 10],
        0x50020000,
    );
    item(&mut out, 1, 0x80, "OK", [174, 142, 50, 14], 0x50010001);
    item(&mut out, 2, 0x80, "Cancel", [230, 142, 50, 14], 0x50010000);
    while out.len() % 4 != 0 {
        out.push(0);
    }
    out.chunks_exact(4)
        .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
        .collect()
}

unsafe fn batch_mode(window: Handle) {
    let numbers = checked(window, 1540);
    for id in [1542, 1543, 1040, 1544, 1547, 1070, 1545] {
        show(window, id, numbers);
    }
    for id in [1546, 1210] {
        show(window, id, !numbers);
    }
}
unsafe fn batch_urls(window: Handle) -> Result<Vec<String>, String> {
    let pattern = get(window, 1007);
    if !pattern.contains('*') {
        return crate::batch::expand_urls(&pattern);
    }
    if pattern.matches('*').count() != 1 {
        return Err("Use one asterisk in the address pattern.".into());
    }
    let range = if checked(window, 1540) {
        let from = get(window, 1542)
            .parse::<u32>()
            .map_err(|_| "Enter a valid starting number.")?;
        let to = get(window, 1040)
            .parse::<u32>()
            .map_err(|_| "Enter a valid ending number.")?;
        let width = get(window, 1070)
            .parse::<usize>()
            .map_err(|_| "Enter a valid wildcard size.")?;
        if !(1..=12).contains(&width) {
            return Err("Wildcard size must be between 1 and 12.".into());
        }
        format!("[{:0width$}-{:0width$}]", from, to)
    } else {
        let from = get(window, 1546);
        let to = get(window, 1210);
        if from.len() != 1
            || to.len() != 1
            || !from.bytes().all(|c| c.is_ascii_alphabetic())
            || !to.bytes().all(|c| c.is_ascii_alphabetic())
        {
            return Err("Enter one letter for each end of the range.".into());
        }
        format!("[{from}-{to}]")
    };
    crate::batch::expand_urls(&pattern.replacen('*', &range, 1))
}
unsafe fn batch_preview(window: Handle) {
    match batch_urls(window) {
        Ok(urls) => {
            text(window, 1548, urls.first().map(String::as_str).unwrap_or(""));
            text(window, 1549, urls.get(1).map(String::as_str).unwrap_or(""));
            text(window, 1550, urls.last().map(String::as_str).unwrap_or(""));
        }
        Err(_) => {
            for id in [1548, 1549, 1550] {
                text(window, id, "");
            }
        }
    }
}
unsafe fn read_auth(window: Handle, form: &mut AddForm) {
    if checked(window, 1182) {
        form.username = get(window, 1033);
        form.password = get(window, 1041);
    } else {
        form.username.clear();
        form.password.clear();
    }
}
unsafe fn auth_enabled(window: Handle) {
    let enabled = checked(window, 1182);
    for id in [1184, 1033, 1186, 1041] {
        enable(window, id, enabled);
    }
}
unsafe fn set_file_icon(window: Handle, control: i32, state: &mut State, name: &str) {
    if !state.file_icon.is_null() {
        DestroyIcon(state.file_icon);
    }
    let mut info: ShellFileInfo = std::mem::zeroed();
    if SHGetFileInfoW(
        wide(if name.is_empty() {
            "download.bin"
        } else {
            name
        })
        .as_ptr(),
        0x80,
        &mut info,
        std::mem::size_of::<ShellFileInfo>() as u32,
        0x100 | 0x10,
    ) != 0
    {
        state.file_icon = info.icon;
        SendMessageW(GetDlgItem(window, control), 0x0170, info.icon as usize, 0);
    } else {
        state.file_icon = null_mut();
    }
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
unsafe fn text(window: Handle, id: i32, value: &str) {
    SetDlgItemTextW(window, id, wide(value).as_ptr());
}
unsafe fn window_text(window: Handle) -> String {
    let length = GetWindowTextLengthW(window).clamp(0, 1_048_576);
    let mut value = vec![0u16; length as usize + 1];
    let count = GetWindowTextW(window, value.as_mut_ptr(), value.len() as i32);
    String::from_utf16_lossy(&value[..count.max(0) as usize])
}
unsafe fn get(window: Handle, id: i32) -> String {
    window_text(GetDlgItem(window, id))
}
unsafe fn enable(window: Handle, id: i32, enabled: bool) {
    EnableWindow(GetDlgItem(window, id), i32::from(enabled));
}
unsafe fn show(window: Handle, id: i32, visible: bool) {
    ShowWindow(GetDlgItem(window, id), if visible { 5 } else { 0 });
}
unsafe fn check(window: Handle, id: i32, checked: bool) {
    SendMessageW(GetDlgItem(window, id), 0x00f1, usize::from(checked), 0);
}
unsafe fn checked(window: Handle, id: i32) -> bool {
    SendMessageW(GetDlgItem(window, id), 0x00f0, 0, 0) == 1
}
unsafe fn combo_add(window: Handle, id: i32, value: &str) {
    SendMessageW(
        GetDlgItem(window, id),
        0x0143,
        0,
        wide(value).as_ptr() as isize,
    );
}
unsafe fn combo_select(window: Handle, id: i32, index: usize) {
    SendMessageW(GetDlgItem(window, id), 0x014e, index, 0);
}
unsafe fn combo_index(window: Handle, id: i32) -> usize {
    SendMessageW(GetDlgItem(window, id), 0x0147, 0, 0).max(0) as usize
}
unsafe fn notice(window: Handle, message: &str) {
    MessageBoxW(
        window,
        wide(message).as_ptr(),
        wide("IDX – Internet Downloader X").as_ptr(),
        0x40,
    );
}
