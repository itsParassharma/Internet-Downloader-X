// SPDX-License-Identifier: MIT
//! Windows shell built from the extracted main dialog (102) and menu (129).
//! The toolbar is created at runtime, as it was in the supplied application;
//! its normal/hot pixels and category images are the extracted original artwork.
//! Download behavior is supplied by the independent Rust controller.
#![allow(unsafe_op_in_unsafe_fn)]

use super::{
    IdxApp, SortColumn, compare_downloads, native_columns, native_forms, native_resources,
    native_settings,
};
use crate::model::{CATEGORIES, Download, Status, human_bytes, human_time};
use chrono::Local;
use eframe::egui;
use std::{
    cell::Cell,
    ffi::c_void,
    net::TcpListener,
    path::PathBuf,
    ptr::{null, null_mut},
};

type Handle = *mut c_void;
type DialogProc = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;
const WM_COMMAND: u32 = 0x111;
const LVM: u32 = 0x1000;
const TVM: u32 = 0x1100;
const TB: u32 = 0x400;
const OPEN_FILE: u32 = 40001;
const OPEN_FOLDER: u32 = 40002;
const PROPERTIES: u32 = 40003;
const PROGRESS: u32 = 40004;
const COPY_URL: u32 = 40005;
const SELECT_ALL: u32 = 40006;
const QUEUE_MOVE: u32 = 42000;
const COMMANDS: [u32; 11] = [
    32771, 32776, 32774, 32779, 32775, 32794, 32781, 32801, 34026, 34028, 33010,
];
const LABELS: [&str; 11] = [
    "Add URL",
    "Resume",
    "Stop",
    "Stop All",
    "Delete",
    "Delete Completed",
    "Options",
    "Scheduler",
    "Start Queue",
    "Stop Queue",
    "Grabber",
];
// Native subitem IDs remain stable for existing saved preferences. The display
// order, labels and widths come from the verified original definitions.
const COLUMNS: [SortColumn; 12] = [
    SortColumn::Filename,
    SortColumn::Queue,
    SortColumn::Size,
    SortColumn::Status,
    SortColumn::Remaining,
    SortColumn::Speed,
    SortColumn::Date,
    SortColumn::Description,
    SortColumn::Added,
    SortColumn::Folder,
    SortColumn::Referer,
    SortColumn::ParentPage,
];

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
#[repr(C)]
struct Message {
    hwnd: Handle,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    point: Point,
    private: u32,
}
#[repr(C)]
struct InitControls {
    size: u32,
    classes: u32,
}
#[repr(C)]
struct Notify {
    hwnd: Handle,
    id: usize,
    code: u32,
}
#[repr(C)]
struct ListNotify {
    header: Notify,
    item: i32,
    subitem: i32,
    new_state: u32,
    old_state: u32,
    changed: u32,
    point: Point,
    param: isize,
}
#[repr(C)]
struct ListColumn {
    mask: u32,
    format: i32,
    width: i32,
    text: *mut u16,
    text_max: i32,
    subitem: i32,
    image: i32,
    order: i32,
    min_width: i32,
    default_width: i32,
    ideal_width: i32,
}
#[repr(C)]
struct ListItem {
    mask: u32,
    item: i32,
    subitem: i32,
    state: u32,
    state_mask: u32,
    text: *mut u16,
    text_max: i32,
    image: i32,
    param: isize,
    indent: i32,
    group_id: i32,
    columns: u32,
    column_indices: *mut u32,
    column_formats: *mut i32,
    group: i32,
}
#[repr(C)]
struct TreeItem {
    mask: u32,
    item: Handle,
    state: u32,
    state_mask: u32,
    text: *mut u16,
    text_max: i32,
    image: i32,
    selected_image: i32,
    children: i32,
    param: isize,
}
#[repr(C)]
struct TreeInsert {
    parent: Handle,
    after: Handle,
    item: TreeItem,
}
#[repr(C)]
struct TreeNotify {
    header: Notify,
    action: u32,
    old_item: TreeItem,
    new_item: TreeItem,
    point: Point,
}
#[repr(C)]
struct ToolbarButton {
    bitmap: i32,
    command: i32,
    state: u8,
    style: u8,
    reserved: [u8; 6],
    data: usize,
    string: isize,
}
#[repr(C)]
struct BitmapHeader {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bits: u16,
    compression: u32,
    image_size: u32,
    x_pixels: i32,
    y_pixels: i32,
    colors_used: u32,
    colors_important: u32,
}
#[repr(C)]
struct BitmapInfo {
    header: BitmapHeader,
    colors: [u32; 3],
}
#[repr(C)]
struct MinMaxInfo {
    reserved: Point,
    max_size: Point,
    max_position: Point,
    min_track: Point,
    max_track: Point,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn CreateDialogIndirectParamW(
        instance: Handle,
        template: *const c_void,
        owner: Handle,
        proc: Option<DialogProc>,
        param: isize,
    ) -> Handle;
    fn CreateWindowExW(
        exstyle: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Handle,
        menu: Handle,
        instance: Handle,
        param: Handle,
    ) -> Handle;
    fn DestroyWindow(window: Handle) -> i32;
    fn GetMessageW(message: *mut Message, window: Handle, minimum: u32, maximum: u32) -> i32;
    fn TranslateMessage(message: *const Message) -> i32;
    fn DispatchMessageW(message: *const Message) -> isize;
    fn IsDialogMessageW(window: Handle, message: *mut Message) -> i32;
    fn PostQuitMessage(code: i32);
    fn SetWindowLongPtrW(window: Handle, index: i32, value: isize) -> isize;
    fn GetWindowLongPtrW(window: Handle, index: i32) -> isize;
    fn GetDlgItem(window: Handle, id: i32) -> Handle;
    fn SetWindowTextW(window: Handle, text: *const u16) -> i32;
    fn SendMessageW(window: Handle, message: u32, w_param: usize, l_param: isize) -> isize;
    fn PostMessageW(window: Handle, message: u32, w_param: usize, l_param: isize) -> i32;
    fn ShowWindow(window: Handle, command: i32) -> i32;
    fn SetForegroundWindow(window: Handle) -> i32;
    fn SetFocus(window: Handle) -> Handle;
    fn GetKeyState(key: i32) -> i16;
    fn SetWindowPos(
        window: Handle,
        after: Handle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    fn GetClientRect(window: Handle, rect: *mut Rect) -> i32;
    fn MapDialogRect(window: Handle, rect: *mut Rect) -> i32;
    fn SetTimer(window: Handle, id: usize, milliseconds: u32, callback: *const c_void) -> usize;
    fn KillTimer(window: Handle, id: usize) -> i32;
    fn LoadImageW(
        instance: Handle,
        name: *const u16,
        image_type: u32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> Handle;
    fn InvalidateRect(window: Handle, rect: *const Rect, erase: i32) -> i32;
    fn LoadMenuIndirectW(template: *const c_void) -> Handle;
    fn SetMenu(window: Handle, menu: Handle) -> i32;
    fn DrawMenuBar(window: Handle) -> i32;
    fn GetMenuItemCount(menu: Handle) -> i32;
    fn GetMenuItemID(menu: Handle, position: i32) -> u32;
    fn GetSubMenu(menu: Handle, position: i32) -> Handle;
    fn GetMenuStringW(menu: Handle, item: u32, text: *mut u16, maximum: i32, flags: u32) -> i32;
    fn ModifyMenuW(menu: Handle, item: u32, flags: u32, id: usize, text: *const u16) -> i32;
    fn DeleteMenu(menu: Handle, item: u32, flags: u32) -> i32;
    fn EnableMenuItem(menu: Handle, item: u32, flags: u32) -> u32;
    fn CheckMenuItem(menu: Handle, item: u32, flags: u32) -> u32;
    fn CreatePopupMenu() -> Handle;
    fn AppendMenuW(menu: Handle, flags: u32, id: usize, text: *const u16) -> i32;
    fn DestroyMenu(menu: Handle) -> i32;
    fn TrackPopupMenu(
        menu: Handle,
        flags: u32,
        x: i32,
        y: i32,
        reserved: i32,
        window: Handle,
        rect: *const Rect,
    ) -> i32;
    fn GetCursorPos(point: *mut Point) -> i32;
    fn MessageBoxW(window: Handle, message: *const u16, title: *const u16, flags: u32) -> i32;
    fn GetSysColorBrush(index: i32) -> Handle;
    fn SetThreadDpiAwarenessContext(context: Handle) -> Handle;
    fn GetDpiForWindow(window: Handle) -> u32;
    fn OpenClipboard(window: Handle) -> i32;
    fn CloseClipboard() -> i32;
    fn GetClipboardData(format: u32) -> Handle;
    fn EmptyClipboard() -> i32;
    fn SetClipboardData(format: u32, memory: Handle) -> Handle;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> Handle;
    fn GetLastError() -> u32;
    fn GlobalAlloc(flags: u32, size: usize) -> Handle;
    fn GlobalLock(memory: Handle) -> Handle;
    fn GlobalUnlock(memory: Handle) -> i32;
    fn GlobalFree(memory: Handle) -> Handle;
    fn GlobalSize(memory: Handle) -> usize;
}
#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateDIBSection(
        dc: Handle,
        info: *const BitmapInfo,
        usage: u32,
        bits: *mut Handle,
        section: Handle,
        offset: u32,
    ) -> Handle;
    fn DeleteObject(object: Handle) -> i32;
}
#[link(name = "comctl32")]
unsafe extern "system" {
    fn InitCommonControlsEx(controls: *const InitControls) -> i32;
    fn ImageList_Create(width: i32, height: i32, flags: u32, initial: i32, grow: i32) -> Handle;
    fn ImageList_Add(list: Handle, image: Handle, mask: Handle) -> i32;
    fn ImageList_Destroy(list: Handle) -> i32;
}
#[link(name = "uxtheme")]
unsafe extern "system" {
    fn SetWindowTheme(window: Handle, app: *const u16, ids: *const u16) -> i32;
}

struct WindowState {
    app: IdxApp,
    context: egui::Context,
    window: Handle,
    toolbar: Handle,
    list: Handle,
    tree: Handle,
    status: Handle,
    menu: Handle,
    images: Vec<Handle>,
    tree_filters: Vec<String>,
    queue_names: Vec<String>,
    category_names: Vec<String>,
    rows: Vec<u64>,
    cells: Vec<Vec<String>>,
    displayed_selection: Option<u64>,
    dpi: u32,
    last_limit: u64,
    error: Option<String>,
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
fn send(window: Handle, message: u32, w: usize, l: isize) -> isize {
    unsafe { SendMessageW(window, message, w, l) }
}
fn set_text(window: Handle, text: &str) {
    unsafe {
        SetWindowTextW(window, wide(text).as_ptr());
    }
}
fn child(window: Handle, id: i32) -> Handle {
    unsafe { GetDlgItem(window, id) }
}
fn px(value: i32, dpi: u32) -> i32 {
    (value * dpi as i32 + 48) / 96
}
fn dialog_rect(window: Handle, x: i32, y: i32, width: i32, height: i32) -> Rect {
    let mut rect = Rect {
        left: x,
        top: y,
        right: x + width,
        bottom: y + height,
    };
    unsafe {
        MapDialogRect(window, &mut rect);
    }
    rect
}

pub fn run(path: PathBuf, listener: TcpListener) -> Result<(), String> {
    unsafe {
        SetThreadDpiAwarenessContext((-4isize) as Handle);
        let controls = InitControls {
            size: std::mem::size_of::<InitControls>() as u32,
            classes: 0xffff,
        };
        if InitCommonControlsEx(&controls) == 0 {
            return Err("Could not initialize Windows controls.".into());
        }
    }
    let context = egui::Context::default();
    let app = IdxApp::from_context(&context, path, listener);
    let mut state = Box::new(WindowState {
        last_limit: app.lib.settings.speed_limit_kib.max(512),
        app,
        context,
        window: null_mut(),
        toolbar: null_mut(),
        list: null_mut(),
        tree: null_mut(),
        status: null_mut(),
        menu: null_mut(),
        images: vec![],
        tree_filters: vec![],
        queue_names: vec![],
        category_names: vec![],
        rows: vec![],
        cells: vec![],
        displayed_selection: None,
        dpi: 96,
        error: None,
    });
    let mut template = native_resources::dialog(102);
    // Keep the original resizable top-level style, adding child-paint clipping.
    if let Some(style) = template.get_mut(3) {
        *style = (*style | 0x0200_0000 | 0x00cf_0000) & !0x1000_0000;
    }
    let window = unsafe {
        CreateDialogIndirectParamW(
            GetModuleHandleW(null()),
            template.as_ptr().cast(),
            null_mut(),
            Some(window_proc),
            (&mut *state as *mut WindowState) as isize,
        )
    };
    if window.is_null() {
        return Err(format!(
            "Could not create the extracted main window (Windows error {}).",
            unsafe { GetLastError() }
        ));
    }
    if let Some(error) = state.error.take() {
        unsafe {
            DestroyWindow(window);
        }
        return Err(error);
    }
    unsafe {
        ShowWindow(window, 5);
        SetForegroundWindow(window);
        SetFocus(state.list);
    }
    let mut message: Message = unsafe { std::mem::zeroed() };
    loop {
        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if result == 0 {
            break;
        }
        if result < 0 {
            state.error = Some("The Windows message loop failed.".into());
            break;
        }
        if shortcut(window, &message) {
            continue;
        }
        unsafe {
            if IsDialogMessageW(window, &mut message) == 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
    state.app.pause_all();
    state.app.save();
    for image in state.images.drain(..) {
        unsafe {
            ImageList_Destroy(image);
        }
    }
    state.error.map_or(Ok(()), Err)
}

fn shortcut(window: Handle, message: &Message) -> bool {
    if message.message != 0x100 {
        return false;
    }
    let ctrl = unsafe { GetKeyState(0x11) } < 0;
    let command = match (ctrl, message.w_param as u32) {
        (true, 0x4e) => 32771,
        (true, 0x42) => 34024,
        (true, 0x41) => SELECT_ALL,
        (true, 0x46) => 34040,
        (true, 0x43) => COPY_URL,
        (false, 0x72) => 34041,
        (false, 0x2e) => 32775,
        (false, 0x0d) => PROGRESS,
        (false, 0x70) => 57670,
        _ => return false,
    };
    unsafe {
        PostMessageW(window, WM_COMMAND, command as usize, 0);
    }
    true
}

unsafe extern "system" fn window_proc(window: Handle, message: u32, w: usize, l: isize) -> isize {
    // DestroyWindow dispatches WM_DESTROY synchronously, including while a
    // timer/command holds the controller. This path needs no controller borrow.
    if message == 0x0002 {
        KillTimer(window, 1);
        PostQuitMessage(0);
        return 1;
    }
    if DISPATCHING.with(|busy| busy.replace(true)) {
        return 0;
    }
    let _guard = DispatchGuard;
    if message == 0x110 {
        SetWindowLongPtrW(window, -21, l);
    }
    let pointer = GetWindowLongPtrW(window, -21) as *mut WindowState;
    if pointer.is_null() {
        return 0;
    }
    // Native modal loops can dispatch this procedure while a sibling dialog has
    // borrowed the controller. Never form a second mutable reference in that case.
    let state = &mut *pointer;
    let result = match message {
        0x110 => {
            state.window = window;
            state.initialize();
            1
        }
        0x113 => {
            state.tick();
            1
        }
        0x111 => {
            state.command((w & 0xffff) as u32);
            1
        }
        0x004e => {
            state.notify(l);
            1
        }
        0x0005 => {
            state.layout();
            1
        }
        0x0010 => {
            state.close();
            1
        }
        0x0024 => {
            let info = &mut *(l as *mut MinMaxInfo);
            info.min_track = Point {
                x: px(760, state.dpi),
                y: px(420, state.dpi),
            };
            1
        }
        0x02e0 => {
            let rect = &*(l as *const Rect);
            state.dpi = (w & 0xffff) as u32;
            SetWindowPos(
                window,
                null_mut(),
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                0x14,
            );
            state.layout();
            1
        }
        0x0117 => {
            state.update_commands();
            1
        }
        0x0136 => GetSysColorBrush(15) as isize,
        _ => 0,
    };
    result
}

thread_local! { static DISPATCHING: Cell<bool> = const { Cell::new(false) }; }
struct DispatchGuard;
impl Drop for DispatchGuard {
    fn drop(&mut self) {
        DISPATCHING.with(|busy| busy.set(false));
    }
}

impl WindowState {
    fn initialize(&mut self) {
        self.dpi = unsafe { GetDpiForWindow(self.window) }.max(96);
        self.list = child(self.window, 1002);
        self.tree = child(self.window, 1276);
        set_text(self.window, "IDX – Internet Downloader X");
        unsafe {
            let instance = GetModuleHandleW(null());
            for (kind, size) in [(0, 16), (1, 32)] {
                let icon = LoadImageW(
                    instance,
                    101usize as *const u16,
                    1,
                    px(size, self.dpi),
                    px(size, self.dpi),
                    0x8000,
                );
                SendMessageW(self.window, 0x80, kind, icon as isize);
            }
            let menu_template = native_resources::menu(129);
            self.menu = LoadMenuIndirectW(menu_template.as_ptr().cast());
            if self.menu.is_null() {
                self.error = Some(format!(
                    "Could not load the extracted menu (Windows error {}).",
                    GetLastError()
                ));
                return;
            }
            sanitize_menu(self.menu);
            SetMenu(self.window, self.menu);
            DrawMenuBar(self.window);
            // BS_OWNERDRAW in resource 102 was backed by original executable code.
            // A native flat pushbutton keeps the same control, label and behavior.
            let close_button = child(self.window, 1278);
            let style = GetWindowLongPtrW(close_button, -16) as u32;
            SetWindowLongPtrW(close_button, -16, ((style & !0xf) | 0x8000) as isize);
            SendMessageW(close_button, 0xf4, 0x8000, 1);
            self.toolbar = CreateWindowExW(
                0,
                wide("ToolbarWindow32").as_ptr(),
                null(),
                0x5000_0000 | 0x0800 | 0x0100 | 0x0040 | 0x000c,
                0,
                0,
                0,
                0,
                self.window,
                19001usize as Handle,
                instance,
                null_mut(),
            );
            self.status = CreateWindowExW(
                0,
                wide("msctls_statusbar32").as_ptr(),
                null(),
                0x5000_0000 | 0x0100,
                0,
                0,
                0,
                0,
                self.window,
                19002usize as Handle,
                instance,
                null_mut(),
            );
            if self.toolbar.is_null() || self.status.is_null() {
                self.error = Some("Could not create the native toolbar or status bar.".into());
                return;
            }
            let font = SendMessageW(self.window, 0x31, 0, 0);
            for control in [self.toolbar, self.status, self.list, self.tree] {
                SendMessageW(control, 0x30, font as usize, 1);
            }
            SetWindowTheme(self.list, wide("Explorer").as_ptr(), null());
            SetWindowTheme(self.tree, wide("").as_ptr(), wide("").as_ptr());
            SendMessageW(self.list, LVM + 54, 0, 0x0001_4022);
            SendMessageW(self.list, LVM + 1, 0, 0x00ff_ffff);
            SendMessageW(self.list, LVM + 38, 0, 0x00ff_ffff);
            // Native tree keeps original line/button styles and white background.
            SendMessageW(self.tree, TVM + 29, 0, 0x00ff_ffff);
        }
        self.create_images();
        self.create_toolbar();
        self.create_columns();
        self.rebuild_tree();
        unsafe {
            SetWindowPos(
                self.window,
                null_mut(),
                80,
                80,
                px(1060, self.dpi),
                px(620, self.dpi),
                0x14,
            );
        }
        self.layout();
        self.refresh();
        unsafe {
            SetTimer(self.window, 1, 180, null());
        }
    }

    fn create_images(&mut self) {
        for (source, message, disabled) in [
            (
                include_bytes!("../../assets/idm/toolbar_normal.rgba").as_slice(),
                TB + 48,
                false,
            ),
            (
                include_bytes!("../../assets/idm/toolbar_hot.rgba").as_slice(),
                TB + 52,
                false,
            ),
            (
                include_bytes!("../../assets/idm/toolbar_normal.rgba").as_slice(),
                TB + 54,
                true,
            ),
        ] {
            let width = px(56, self.dpi);
            let height = px(42, self.dpi);
            let list = unsafe { ImageList_Create(width, height, 0x20, 12, 1) };
            if list.is_null() {
                continue;
            }
            for tile in 0..12 {
                let mut rgba = Vec::with_capacity(56 * 42 * 4);
                for y in 0..42 {
                    let start = (y * 672 + tile * 56) * 4;
                    rgba.extend_from_slice(&source[start..start + 56 * 4]);
                }
                if disabled {
                    for pixel in rgba.chunks_exact_mut(4) {
                        let gray =
                            ((77 * pixel[0] as u16 + 150 * pixel[1] as u16 + 29 * pixel[2] as u16)
                                >> 8) as u8;
                        pixel[0] = gray;
                        pixel[1] = gray;
                        pixel[2] = gray;
                        pixel[3] = (pixel[3] as u16 * 145 / 255) as u8;
                    }
                }
                add_image(list, &rgba, 56, 42, width, height);
            }
            send(self.toolbar, message, 0, list as isize);
            self.images.push(list);
        }
        let list = unsafe { ImageList_Create(px(16, self.dpi), px(13, self.dpi), 0x20, 11, 1) };
        if !list.is_null() {
            let sources: [&[u8]; 11] = [
                include_bytes!("../../assets/idm/sidebar_all.rgba"),
                include_bytes!("../../assets/idm/sidebar_unfinished.rgba"),
                include_bytes!("../../assets/idm/sidebar_finished.rgba"),
                include_bytes!("../../assets/idm/sidebar_compressed.rgba"),
                include_bytes!("../../assets/idm/sidebar_documents.rgba"),
                include_bytes!("../../assets/idm/sidebar_music.rgba"),
                include_bytes!("../../assets/idm/sidebar_programs.rgba"),
                include_bytes!("../../assets/idm/sidebar_video.rgba"),
                include_bytes!("../../assets/idm/sidebar_other.rgba"),
                include_bytes!("../../assets/idm/sidebar_queues.rgba"),
                include_bytes!("../../assets/idm/sidebar_grabber.rgba"),
            ];
            for source in sources {
                add_image(list, source, 16, 13, px(16, self.dpi), px(13, self.dpi));
            }
            send(self.tree, TVM + 9, 0, list as isize);
            send(self.list, LVM + 3, 1, list as isize);
            self.images.push(list);
        }
    }

    fn create_toolbar(&self) {
        send(
            self.toolbar,
            TB + 30,
            std::mem::size_of::<ToolbarButton>(),
            0,
        );
        send(self.toolbar, TB + 84, 0, 0x80); // double buffered toolbar
        send(self.toolbar, TB + 60, 1, 0); // one line of labels
        send(self.toolbar, 0x2001, 0, 0x00ff_ffff);
        let mut buttons = Vec::new();
        for index in 0..COMMANDS.len() {
            if index == 4 || index == 6 {
                buttons.push(ToolbarButton {
                    bitmap: px(7, self.dpi),
                    command: 0,
                    state: 4,
                    style: 1,
                    reserved: [0; 6],
                    data: 0,
                    string: 0,
                });
            }
            let mut text = wide(LABELS[index]);
            text.push(0);
            let string = send(self.toolbar, TB + 77, 0, text.as_ptr() as isize);
            buttons.push(ToolbarButton {
                bitmap: index as i32,
                command: COMMANDS[index] as i32,
                state: 4,
                style: 0x10 | 0x40,
                reserved: [0; 6],
                data: 0,
                string,
            });
        }
        send(
            self.toolbar,
            TB + 68,
            buttons.len(),
            buttons.as_ptr() as isize,
        );
        send(
            self.toolbar,
            TB + 31,
            0,
            ((px(66, self.dpi) as u32) << 16 | px(68, self.dpi) as u32) as isize,
        );
        send(self.toolbar, TB + 33, 0, 0);
    }

    fn create_columns(&mut self) {
        for (index, title) in crate::columns::TITLES.iter().enumerate() {
            let width = crate::columns::DEFAULTS
                .iter()
                .find(|column| column.0 == index)
                .unwrap()
                .2;
            let mut text = wide(title);
            let column = ListColumn {
                mask: 0x7,
                format: if index == 2 { 1 } else { 0 },
                width: px(width as i32, self.dpi),
                text: text.as_mut_ptr(),
                text_max: 0,
                subitem: index as i32,
                image: 0,
                order: index as i32,
                min_width: 0,
                default_width: 0,
                ideal_width: 0,
            };
            send(self.list, LVM + 97, index, &column as *const _ as isize);
        }
        self.apply_columns();
    }

    fn apply_columns(&mut self) {
        let defaults = &crate::columns::DEFAULTS;
        self.app.lib.settings.columns =
            crate::columns::normalize(&self.app.lib.settings.columns, defaults);
        let mut order = Vec::with_capacity(COLUMNS.len());
        for &(id, visible, width) in &self.app.lib.settings.columns {
            order.push(id as i32);
            send(
                self.list,
                LVM + 30,
                id,
                if visible {
                    crate::columns::to_pixels(width, self.dpi) as isize
                } else {
                    0
                },
            );
        }
        send(self.list, LVM + 58, order.len(), order.as_ptr() as isize);
    }

    fn capture_columns(&mut self) {
        let defaults = &crate::columns::DEFAULTS;
        let current = crate::columns::normalize(&self.app.lib.settings.columns, defaults);
        let mut order: Vec<i32> = (0..COLUMNS.len() as i32).collect();
        if send(
            self.list,
            LVM + 59,
            order.len(),
            order.as_mut_ptr() as isize,
        ) == 0
        {
            order = current.iter().map(|column| column.0 as i32).collect();
        }
        let mut captured = Vec::with_capacity(order.len());
        for id in order {
            let Some(&(_, _, old_width)) = current.iter().find(|column| column.0 == id as usize)
            else {
                continue;
            };
            let pixels = send(self.list, LVM + 29, id as usize, 0).max(0) as u32;
            let width = if pixels == 0 || pixels == crate::columns::to_pixels(old_width, self.dpi) {
                old_width
            } else {
                crate::columns::to_logical(pixels, self.dpi)
            };
            captured.push((id as usize, pixels > 0, width));
        }
        self.app.lib.settings.columns = crate::columns::normalize(&captured, defaults);
    }

    fn insert_tree(&mut self, parent: Handle, title: &str, filter: String, image: i32) -> Handle {
        let index = self.tree_filters.len();
        self.tree_filters.push(filter);
        let mut text = wide(title);
        let insert = TreeInsert {
            parent,
            after: (-0x0fffeisize) as Handle,
            item: TreeItem {
                mask: 0x1 | 0x2 | 0x20 | 0x4,
                item: null_mut(),
                state: 0,
                state_mask: 0,
                text: text.as_mut_ptr(),
                text_max: 0,
                image,
                selected_image: image,
                children: 0,
                param: index as isize,
            },
        };
        send(self.tree, TVM + 50, 0, &insert as *const _ as isize) as Handle
    }

    fn rebuild_tree(&mut self) {
        send(self.tree, 0x000b, 0, 0);
        send(self.tree, TVM + 1, 0, -0x10000);
        self.tree_filters.clear();
        let root = (-0x10000isize) as Handle;
        let all = self.insert_tree(root, "All Downloads", "All downloads".into(), 0);
        let mut selected = all;
        self.category_names = self
            .app
            .lib
            .settings
            .categories
            .iter()
            .map(|c| c.name.clone())
            .collect();
        for category in self.category_names.clone() {
            let item = self.insert_tree(all, &category, category.clone(), category_icon(&category));
            if self.app.filter == category {
                selected = item;
            }
        }
        for (title, filter, image) in [("Unfinished", "Unfinished", 1), ("Finished", "Finished", 2)]
        {
            let item = self.insert_tree(root, title, filter.into(), image);
            if self.app.filter == filter {
                selected = item;
            }
        }
        let queues = self.insert_tree(root, "Queues", "All downloads".into(), 9);
        self.queue_names = self
            .app
            .lib
            .settings
            .queues
            .iter()
            .map(|q| q.name.clone())
            .collect();
        for queue in self.queue_names.clone() {
            let filter = format!("queue:{queue}");
            let item = self.insert_tree(queues, &queue, filter.clone(), 9);
            if self.app.filter == filter {
                selected = item;
            }
        }
        self.insert_tree(root, "Grabber projects", "grabber".into(), 10);
        send(self.tree, TVM + 2, 2, all as isize);
        send(self.tree, TVM + 2, 2, queues as isize);
        send(self.tree, TVM + 11, 9, selected as isize);
        send(self.tree, 0x000b, 1, 0);
        unsafe {
            InvalidateRect(self.tree, null(), 1);
        }
    }

    fn layout(&self) {
        if self.toolbar.is_null() {
            return;
        }
        let mut area = Rect::default();
        unsafe {
            GetClientRect(self.window, &mut area);
        }
        let scale = |value| px(value, self.dpi);
        let top = scale(73);
        let bottom = scale(23);
        let original = dialog_rect(self.window, 0, 0, 532, 209);
        let list_rect = dialog_rect(self.window, 102, 5, 425, 203);
        let tree_width = if self.app.show_categories {
            list_rect.left
        } else {
            0
        };
        let extra_height = area.bottom - top - bottom - original.bottom;
        unsafe {
            SetWindowPos(self.toolbar, null_mut(), 0, 0, area.right, top, 0x14);
            SendMessageW(self.status, 5, 0, 0);
            for id in [1276, 1278, 1279, 1280, 1281] {
                ShowWindow(
                    child(self.window, id),
                    if self.app.show_categories { 5 } else { 0 },
                );
            }
            if self.app.show_categories {
                // These are the exact dialog-unit coordinates from resource102.
                // Only the bottom edge grows with the resizable parent window.
                for (id, x, y, width, height, stretch) in [
                    (1279, 4, 1, 95, 16, false),
                    (1280, 4, 1, 95, 207, true),
                    (1281, 8, 7, 49, 8, false),
                    (1278, 87, 8, 9, 8, false),
                    (1276, 5, 16, 92, 189, true),
                ] {
                    let rect = dialog_rect(self.window, x, y, width, height);
                    SetWindowPos(
                        child(self.window, id),
                        null_mut(),
                        rect.left,
                        top + rect.top,
                        rect.right - rect.left,
                        (rect.bottom - rect.top + if stretch { extra_height } else { 0 }).max(1),
                        0x14,
                    );
                }
            }
            SetWindowPos(
                self.list,
                null_mut(),
                tree_width,
                top + list_rect.top,
                (area.right - tree_width - (original.right - list_rect.right)).max(1),
                (list_rect.bottom - list_rect.top + extra_height).max(1),
                0x14,
            );
        }
        let parts = [
            scale(170),
            scale(290),
            (area.right - scale(230)).max(scale(400)),
            -1,
        ];
        send(self.status, TB + 4, parts.len(), parts.as_ptr() as isize);
    }

    fn tick(&mut self) {
        self.app.tick(&self.context);
        self.app.sync_native_progress();
        if self.app.closing {
            if self.app.controls.is_empty() {
                self.app.save();
                unsafe {
                    DestroyWindow(self.window);
                }
            } else {
                self.status_text(0, "Pausing downloads…");
            }
            return;
        }
        if self.app.focus_requested {
            self.app.focus_requested = false;
            unsafe {
                ShowWindow(self.window, 9);
                SetForegroundWindow(self.window);
            }
        }
        if self.app.add.open {
            native_forms::add(self.window, &mut self.app);
        }
        if self.app.browser_setup_pending {
            self.app.browser_setup_pending = false;
            native_settings::browser_setup(self.window, &mut self.app);
        }
        for id in std::mem::take(&mut self.app.pending_completions) {
            native_forms::complete(self.window, &mut self.app, id);
        }
        if let Some(message) = self.app.message.take() {
            unsafe {
                MessageBoxW(
                    self.window,
                    wide(&message).as_ptr(),
                    wide("IDX – Internet Downloader X").as_ptr(),
                    0x40,
                );
            }
        }
        if self.queue_names
            != self
                .app
                .lib
                .settings
                .queues
                .iter()
                .map(|q| q.name.clone())
                .collect::<Vec<_>>()
            || self.category_names
                != self
                    .app
                    .lib
                    .settings
                    .categories
                    .iter()
                    .map(|c| c.name.clone())
                    .collect::<Vec<_>>()
        {
            self.rebuild_tree();
        }
        self.refresh();
    }

    fn visible(&self) -> Vec<&Download> {
        let query = self.app.search.to_lowercase();
        let mut rows: Vec<_> = self
            .app
            .lib
            .downloads
            .iter()
            .filter(|download| {
                let selected_category = match self.app.filter.as_str() {
                    "All downloads" => true,
                    "Unfinished" => download.status != Status::Complete,
                    "Finished" => download.status == Status::Complete,
                    filter if filter.starts_with("queue:") => download.queue == filter[6..],
                    filter => download.category == filter,
                };
                selected_category
                    && (query.is_empty()
                        || download.name().to_lowercase().contains(&query)
                        || download.url.to_lowercase().contains(&query)
                        || download.description.to_lowercase().contains(&query))
            })
            .collect();
        if let Some(column) = self.app.sort_column {
            rows.sort_by(|left, right| {
                let order = compare_downloads(column, left, right);
                if self.app.sort_descending {
                    order.reverse()
                } else {
                    order
                }
            });
        }
        rows
    }

    fn refresh(&mut self) {
        let visible = self.visible();
        let ids: Vec<_> = visible.iter().map(|download| download.id).collect();
        let values: Vec<_> = visible.iter().map(|download| row_text(download)).collect();
        let icons: Vec<_> = visible
            .iter()
            .map(|download| category_icon(&download.category))
            .collect();
        let queue_icons: Vec<_> = visible
            .iter()
            .map(|download| {
                if download.status == Status::Complete {
                    -2
                } else {
                    9
                }
            })
            .collect();
        let changed_order = ids != self.rows;
        send(self.list, 0xb, 0, 0);
        if changed_order {
            let top = send(self.list, LVM + 39, 0, 0);
            send(self.list, LVM + 9, 0, 0);
            for (index, id) in ids.iter().enumerate() {
                let mut text = wide(&values[index][0]);
                let item = ListItem {
                    mask: 0x1 | 0x2 | 0x4 | 0x8,
                    item: index as i32,
                    subitem: 0,
                    state: if self.app.selected_ids.contains(id) {
                        2
                    } else {
                        0
                    } | if self.app.selected == Some(*id) { 1 } else { 0 },
                    state_mask: 3,
                    text: text.as_mut_ptr(),
                    text_max: 0,
                    image: icons[index],
                    param: *id as isize,
                    indent: 0,
                    group_id: 0,
                    columns: 0,
                    column_indices: null_mut(),
                    column_formats: null_mut(),
                    group: 0,
                };
                send(self.list, LVM + 77, 0, &item as *const _ as isize);
            }
            if top > 0 && (top as usize) < ids.len() {
                send(self.list, LVM + 19, top as usize, 0);
            }
        }
        for (row, cells) in values.iter().enumerate() {
            for (column, text) in cells.iter().enumerate() {
                if changed_order
                    || self.cells.get(row).and_then(|old| old.get(column)) != Some(text)
                {
                    let mut text = wide(text);
                    let item = ListItem {
                        mask: 0x1,
                        item: row as i32,
                        subitem: column as i32,
                        state: 0,
                        state_mask: 0,
                        text: text.as_mut_ptr(),
                        text_max: 0,
                        image: 0,
                        param: 0,
                        indent: 0,
                        group_id: 0,
                        columns: 0,
                        column_indices: null_mut(),
                        column_formats: null_mut(),
                        group: 0,
                    };
                    send(self.list, LVM + 116, row, &item as *const _ as isize);
                }
            }
        }
        // Find and other modal workflows can change selection without changing
        // row order. Apply that state even when the row text has not changed.
        for (row, id) in ids.iter().enumerate() {
            let desired = if self.app.selected_ids.contains(id) {
                2
            } else {
                0
            } | if self.app.selected == Some(*id) { 1 } else { 0 };
            if send(self.list, LVM + 44, row, 3) as u32 != desired {
                let mut item: ListItem = unsafe { std::mem::zeroed() };
                item.state = desired;
                item.state_mask = 3;
                send(self.list, LVM + 43, row, &item as *const _ as isize);
            }
            // Keep category and queue glyphs in the native list's image slots.
            // The bitmap pixels are the original resources, including column Q.
            for (subitem, image) in [(0, icons[row]), (1, queue_icons[row])] {
                let mut item: ListItem = unsafe { std::mem::zeroed() };
                item.mask = 2;
                item.item = row as i32;
                item.subitem = subitem;
                item.image = image;
                send(self.list, LVM + 76, 0, &item as *const _ as isize);
            }
        }
        if self.displayed_selection != self.app.selected {
            if let Some(row) = ids.iter().position(|id| Some(*id) == self.app.selected) {
                send(self.list, LVM + 19, row, 0);
            }
            self.displayed_selection = self.app.selected;
        }
        self.rows = ids;
        self.cells = values;
        send(self.list, 0xb, 1, 0);
        unsafe {
            InvalidateRect(self.list, null(), 0);
        }
        self.status_text(0, &format!("{} downloads", self.app.lib.downloads.len()));
        self.status_text(1, &format!("{} active", self.app.controls.len()));
        let speed: f64 = self
            .app
            .lib
            .downloads
            .iter()
            .map(|download| download.speed)
            .sum();
        self.status_text(
            2,
            &format!("Transfer rate: {}/sec", human_bytes(speed as u64)),
        );
        self.status_text(
            3,
            &if self.app.lib.settings.speed_limit_kib == 0 {
                "Speed limiter: Off".into()
            } else {
                format!(
                    "Speed limiter: {} KB/sec",
                    self.app.lib.settings.speed_limit_kib
                )
            },
        );
        self.update_commands();
    }

    fn status_text(&self, part: usize, text: &str) {
        send(self.status, TB + 11, part, wide(text).as_ptr() as isize);
    }

    fn selection(&mut self) {
        self.app.selected_ids.clear();
        let mut index = -1isize;
        loop {
            index = send(self.list, LVM + 12, index as usize, 2);
            if index < 0 {
                break;
            }
            if let Some(id) = self.rows.get(index as usize) {
                self.app.selected_ids.insert(*id);
            }
        }
        let focused = send(self.list, LVM + 12, usize::MAX, 1);
        self.app.selected = if focused >= 0 {
            self.rows
                .get(focused as usize)
                .copied()
                .filter(|id| self.app.selected_ids.contains(id))
        } else {
            None
        }
        .or_else(|| {
            self.rows
                .iter()
                .copied()
                .find(|id| self.app.selected_ids.contains(id))
        });
        self.app.selection_anchor = self.app.selected;
        self.update_commands();
    }

    fn notify(&mut self, address: isize) {
        if address == 0 {
            return;
        }
        let header = unsafe { &*(address as *const Notify) };
        if header.hwnd == self.list {
            match header.code as i32 {
                -101 => self.selection(), // LVN_ITEMCHANGED
                -108 => {
                    let notice = unsafe { &*(address as *const ListNotify) };
                    if let Some(column) = COLUMNS.get(notice.subitem as usize) {
                        self.sort(*column);
                    }
                }
                -3 => {
                    self.selection();
                    self.command(PROGRESS);
                }
                -5 => {
                    self.selection();
                    self.context_menu();
                }
                _ => (),
            }
        } else if header.hwnd == self.tree && header.code as i32 == -451 {
            let notice = unsafe { &*(address as *const TreeNotify) };
            if let Some(filter) = self
                .tree_filters
                .get(notice.new_item.param as usize)
                .cloned()
            {
                if filter == "grabber" {
                    native_settings::grabber(self.window, &mut self.app);
                } else {
                    self.app.filter = filter;
                    self.refresh();
                }
            }
        }
    }

    fn sort(&mut self, column: SortColumn) {
        if self.app.sort_column == Some(column) {
            self.app.sort_descending = !self.app.sort_descending;
        } else {
            self.app.sort_column = Some(column);
            self.app.sort_descending = false;
        }
        self.refresh();
    }

    fn update_commands(&self) {
        let targets = self.app.selected_targets();
        let selected: Vec<_> = self
            .app
            .lib
            .downloads
            .iter()
            .filter(|download| targets.contains(&download.id))
            .collect();
        let resume = selected.iter().any(|download| {
            !download.status.active()
                && !matches!(download.status, Status::Complete | Status::Verifying)
        });
        let pause = selected
            .iter()
            .any(|download| download.status.active() || download.status == Status::Queued);
        let completed = self
            .app
            .lib
            .downloads
            .iter()
            .any(|download| download.status == Status::Complete);
        for (command, enabled) in [
            (32776, resume),
            (32774, pause),
            (32775, !selected.is_empty()),
            (32778, !self.app.controls.is_empty()),
            (32779, !self.app.controls.is_empty()),
            (32794, completed),
        ] {
            send(self.toolbar, TB + 1, command, i32::from(enabled) as isize);
            unsafe {
                EnableMenuItem(self.menu, command as u32, if enabled { 0 } else { 1 });
            }
        }
        unsafe {
            CheckMenuItem(
                self.menu,
                34033,
                if self.app.lib.settings.speed_limit_kib > 0 {
                    8
                } else {
                    0
                },
            );
            CheckMenuItem(
                self.menu,
                34034,
                if self.app.lib.settings.speed_limit_kib == 0 {
                    8
                } else {
                    0
                },
            );
            CheckMenuItem(self.menu, 34011, 8);
            CheckMenuItem(self.menu, 32841, 8);
        }
    }

    fn command(&mut self, command: u32) {
        if self.app.closing {
            return;
        }
        match command {
            32771 => {
                self.app.add_dialog(String::new());
                native_forms::add(self.window, &mut self.app);
            }
            34024 => native_forms::batch(self.window, &mut self.app),
            34036 => match clipboard(self.window) {
                Some(text) if !text.trim().is_empty() => {
                    self.app.add_dialog(text);
                    native_forms::add(self.window, &mut self.app);
                }
                _ => {
                    self.app.message =
                        Some("The clipboard does not contain any text addresses.".into())
                }
            },
            32772 => self.close(),
            32774 => self.app.pause_selected(),
            32775 => {
                let ids = self.app.selected_targets();
                if !ids.is_empty()
                    && (!self.app.lib.settings.confirm_delete
                        || native_forms::delete(self.window, ids.len()))
                {
                    self.app.remove_many(&ids);
                }
            }
            32776 => self.app.resume_selected(),
            32778 | 32779 => self.app.pause_all(),
            32794 => {
                let ids: Vec<_> = self
                    .app
                    .lib
                    .downloads
                    .iter()
                    .filter(|download| download.status == Status::Complete)
                    .map(|download| download.id)
                    .collect();
                if !ids.is_empty()
                    && (!self.app.lib.settings.confirm_delete
                        || native_forms::delete(self.window, ids.len()))
                {
                    self.app.remove_many(&ids);
                }
            }
            32781 => native_settings::options(self.window, &mut self.app),
            33002 => {
                self.capture_columns();
                let saved = self.app.lib.settings.columns.clone();
                let defaults = &crate::columns::DEFAULTS;
                let titles = &crate::columns::TITLES;
                let context = self.context.clone();
                let result =
                    native_columns::show(self.window, &saved, defaults, titles, self.dpi, || {
                        self.app.tick(&context);
                        self.app.sync_native_progress();
                    });
                match result {
                    Ok(Some(columns)) => {
                        self.app.lib.settings.columns = columns;
                        self.apply_columns();
                        self.app.save();
                    }
                    Ok(None) => {}
                    Err(error) => self.app.message = Some(error),
                }
            }
            34035 => native_forms::speed_limit(self.window, &mut self.app),
            32801 => native_settings::scheduler(self.window, &mut self.app),
            33010 => native_settings::grabber(self.window, &mut self.app),
            32786 => native_forms::about(self.window),
            32818 => self.app.import(),
            32823 => self.app.export(),
            34026 => self.app.start_queue(self.queue_index()),
            34028 => self.app.stop_queue(self.queue_index()),
            34040 => {
                native_forms::find(self.window, &mut self.app);
                self.rebuild_tree();
            }
            34041 => {
                let matches: Vec<_> = self
                    .visible()
                    .iter()
                    .filter(|download| self.app.selected_ids.contains(&download.id))
                    .map(|download| download.id)
                    .collect();
                if matches.is_empty() {
                    native_forms::find(self.window, &mut self.app);
                    self.rebuild_tree();
                } else {
                    // The original Find dialog selects its matching records.
                    // Advance the focus within those matches in displayed order.
                    let current = matches
                        .iter()
                        .position(|id| Some(*id) == self.app.selected)
                        .unwrap_or(matches.len() - 1);
                    self.app.selected = Some(matches[(current + 1) % matches.len()]);
                    self.app.selection_anchor = self.app.selected;
                }
            }
            34033 | 34034 => {
                let limit = self.app.lib.settings.speed_limit_kib;
                if limit > 0 {
                    self.last_limit = limit;
                }
                self.app.lib.settings.speed_limit_kib = if command == 34033 {
                    self.last_limit.max(1)
                } else {
                    0
                };
                for download in &self.app.lib.downloads {
                    if download.speed_limit_kib.is_none() {
                        if let Some(control) = self.app.controls.get(&download.id) {
                            control.set_speed_limit(self.app.lib.settings.speed_limit_kib);
                        }
                    }
                }
                self.app.save();
            }
            1278 | 32825 => {
                self.app.show_categories = !self.app.show_categories;
                self.layout();
            }
            32828 => self.sort(SortColumn::Added),
            32830 => self.sort(SortColumn::Filename),
            32832 => self.sort(SortColumn::Size),
            32834 => self.sort(SortColumn::Status),
            32836 => self.sort(SortColumn::Remaining),
            32838 => self.sort(SortColumn::Speed),
            33006 => self.sort(SortColumn::Date),
            32840 => self.sort(SortColumn::Description),
            33004 => self.sort(SortColumn::Folder),
            33005 => self.sort(SortColumn::Referer),
            34043 => self.sort(SortColumn::ParentPage),
            OPEN_FILE | OPEN_FOLDER => {
                if let Some(download) = self.app.selected.and_then(|id| {
                    self.app
                        .lib
                        .downloads
                        .iter()
                        .find(|download| download.id == id)
                }) {
                    if command == OPEN_FOLDER {
                        if let Some(folder) = download.path.parent() {
                            super::open_path(folder);
                        }
                    } else if download.status == Status::Complete {
                        super::open_path(&download.path);
                    } else {
                        self.app.message = Some("The selected download is not complete.".into());
                    }
                }
            }
            PROPERTIES => native_forms::properties(self.window, &mut self.app),
            PROGRESS => {
                self.app.transfer_open = self.app.selected.is_some();
                self.app.sync_native_progress();
            }
            COPY_URL => {
                let targets = self.app.selected_targets();
                let urls = self
                    .app
                    .lib
                    .downloads
                    .iter()
                    .filter(|download| targets.contains(&download.id))
                    .map(|download| download.url.as_str())
                    .collect::<Vec<_>>()
                    .join("\r\n");
                if !urls.is_empty() {
                    copy_clipboard(self.window, &urls);
                }
            }
            SELECT_ALL => {
                let mut item: ListItem = unsafe { std::mem::zeroed() };
                item.state = 2;
                item.state_mask = 2;
                send(self.list, LVM + 43, usize::MAX, &item as *const _ as isize);
                self.selection();
            }
            57670 | 32811 | 34029 | 34022 => self.help(command),
            value
                if value >= QUEUE_MOVE
                    && (value - QUEUE_MOVE) < self.app.lib.settings.queues.len() as u32 =>
            {
                let name = self.app.lib.settings.queues[(value - QUEUE_MOVE) as usize]
                    .name
                    .clone();
                let targets = self.app.selected_targets();
                for download in &mut self.app.lib.downloads {
                    if targets.contains(&download.id) && !download.status.active() {
                        download.queue = name.clone();
                    }
                }
                self.app.save();
            }
            // Escape should not silently close the main window, and disabled
            // original commands never imply their old behavior was ported.
            _ => (),
        }
        if !self.app.closing {
            self.refresh();
        }
    }

    fn queue_index(&self) -> usize {
        self.app
            .filter
            .strip_prefix("queue:")
            .and_then(|name| {
                self.app
                    .lib
                    .settings
                    .queues
                    .iter()
                    .position(|queue| queue.name == name)
            })
            .unwrap_or(0)
    }

    fn close(&mut self) {
        if !self.app.closing {
            self.capture_columns();
            self.app.closing = true;
            self.app.pause_all();
            self.app.native_progress.hide();
        }
        if self.app.controls.is_empty() {
            self.app.save();
            unsafe {
                DestroyWindow(self.window);
            }
        } else {
            self.status_text(0, "Pausing downloads…");
        }
    }

    fn help(&mut self, command: u32) {
        let doc = if command == 34029 {
            "FEATURE-PARITY.md"
        } else if command == 34022 {
            "FEATURE-PARITY.md"
        } else {
            "BROWSER-INTEGRATION.md"
        };
        if let Ok(executable) = std::env::current_exe() {
            if let Some(folder) = executable.parent() {
                let path = folder.join("docs").join(doc);
                if path.exists() {
                    super::open_path(&path);
                    return;
                }
            }
        }
        self.app.message = Some("Use Add URL to add an HTTP or HTTPS address, then Start Download. Double-click a download for its progress window. Select Scheduler to manage queues. Browser integration instructions are included in docs/BROWSER-INTEGRATION.md beside IDX. Unsupported original features are disabled; see docs/FEATURE-PARITY.md for coverage.".into());
    }

    fn context_menu(&mut self) {
        let menu = unsafe { CreatePopupMenu() };
        if menu.is_null() {
            return;
        }
        for (id, label) in [
            (32776, "Resume Download"),
            (32774, "Stop Download"),
            (0, ""),
            (OPEN_FILE, "Open"),
            (OPEN_FOLDER, "Open folder"),
            (PROGRESS, "Download progress"),
            (PROPERTIES, "Properties"),
            (COPY_URL, "Copy URL"),
            (0, ""),
            (32775, "Remove"),
        ] {
            unsafe {
                AppendMenuW(
                    menu,
                    if id == 0 { 0x800 } else { 0 },
                    id as usize,
                    wide(label).as_ptr(),
                );
            }
        }
        let queues = unsafe { CreatePopupMenu() };
        for (index, queue) in self.app.lib.settings.queues.iter().enumerate() {
            unsafe {
                AppendMenuW(
                    queues,
                    0,
                    QUEUE_MOVE as usize + index,
                    wide(&queue.name).as_ptr(),
                );
            }
        }
        unsafe {
            AppendMenuW(menu, 0x10, queues as usize, wide("Move to queue").as_ptr());
            let mut point = Point::default();
            GetCursorPos(&mut point);
            let action = TrackPopupMenu(menu, 0x102, point.x, point.y, 0, self.window, null());
            DestroyMenu(menu);
            if action != 0 {
                self.command(action as u32);
            }
        }
    }
}

fn category_icon(category: &str) -> i32 {
    CATEGORIES
        .iter()
        .position(|value| *value == category)
        .map_or(8, |index| index as i32 + 3)
}

fn row_text(download: &Download) -> Vec<String> {
    vec![
        download.name(),
        String::new(),
        download
            .total
            .map(human_bytes)
            .unwrap_or_else(|| "—".into()),
        if download.status == Status::Downloading {
            format!("{:.2}%", download.percent() * 100.0)
        } else {
            download.status.label().into()
        },
        if download.speed > 0.0 {
            human_time(
                download
                    .total
                    .unwrap_or(download.downloaded)
                    .saturating_sub(download.downloaded) as f64
                    / download.speed,
            )
        } else {
            String::new()
        },
        if download.speed > 0.0 {
            format!("{}/sec", human_bytes(download.speed as u64))
        } else {
            String::new()
        },
        download
            .last_try
            .map(|date| {
                date.with_timezone(&Local)
                    .format("%d %b %Y %H:%M")
                    .to_string()
            })
            .unwrap_or_default(),
        download.description.clone(),
        download
            .added
            .with_timezone(&Local)
            .format("%d %b %Y %H:%M")
            .to_string(),
        download
            .path
            .parent()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default(),
        download.referer.clone(),
        download.parent_page.clone(),
    ]
}

unsafe fn sanitize_menu(menu: Handle) {
    // These are existing Rust operations. Everything else retains its extracted
    // caption and placement but is visibly unavailable, rather than a dead button.
    let implemented: &[u32] = &[
        32771, 34024, 34036, 33010, 32823, 32818, 32772, 32774, 32775, 32776, 32778, 32779, 32794,
        34040, 34041, 32801, 34026, 34028, 34033, 34034, 34035, 32781, 32825, 32828, 32830, 32832,
        32834, 32836, 32838, 33006, 32840, 33004, 33005, 34043, 33002, 34011, 32841, 57670, 32811,
        34029, 34022, 32786,
    ];
    for position in (0..GetMenuItemCount(menu)).rev() {
        let mut caption = [0u16; 256];
        let length = GetMenuStringW(
            menu,
            position as u32,
            caption.as_mut_ptr(),
            caption.len() as i32,
            0x400,
        )
        .max(0) as usize;
        let caption = String::from_utf16_lossy(&caption[..length]);
        let id = GetMenuItemID(menu, position);
        if matches!(id, 32783 | 32785)
            || caption
                .replace('&', "")
                .eq_ignore_ascii_case("Registration")
        {
            DeleteMenu(menu, position as u32, 0x400);
            continue;
        }
        let submenu = GetSubMenu(menu, position);
        let rebranded = caption
            .replace("Internet Download Manager", "Internet Downloader X")
            .replace("IDM", "IDX");
        if rebranded != caption {
            ModifyMenuW(
                menu,
                position as u32,
                0x400 | if submenu.is_null() { 0 } else { 0x10 },
                if submenu.is_null() {
                    id as usize
                } else {
                    submenu as usize
                },
                wide(&rebranded).as_ptr(),
            );
        }
        if !submenu.is_null() {
            sanitize_menu(submenu);
        } else if id != 0 && !implemented.contains(&id) {
            EnableMenuItem(menu, position as u32, 0x400 | 1);
        }
    }
}

fn add_image(
    list: Handle,
    rgba: &[u8],
    source_width: usize,
    source_height: usize,
    width: i32,
    height: i32,
) {
    let info = BitmapInfo {
        header: BitmapHeader {
            size: 40,
            width,
            height: -height,
            planes: 1,
            bits: 32,
            compression: 0,
            image_size: 0,
            x_pixels: 0,
            y_pixels: 0,
            colors_used: 0,
            colors_important: 0,
        },
        colors: [0; 3],
    };
    let mut bits = null_mut();
    let bitmap = unsafe { CreateDIBSection(null_mut(), &info, 0, &mut bits, null_mut(), 0) };
    if bitmap.is_null() || bits.is_null() {
        return;
    }
    let pixels = unsafe {
        std::slice::from_raw_parts_mut(bits.cast::<u8>(), width as usize * height as usize * 4)
    };
    for y in 0..height as usize {
        for x in 0..width as usize {
            let source = ((y * source_height / height as usize) * source_width
                + x * source_width / width as usize)
                * 4;
            let target = (y * width as usize + x) * 4;
            let alpha = rgba[source + 3] as u16;
            pixels[target] = (rgba[source + 2] as u16 * alpha / 255) as u8;
            pixels[target + 1] = (rgba[source + 1] as u16 * alpha / 255) as u8;
            pixels[target + 2] = (rgba[source] as u16 * alpha / 255) as u8;
            pixels[target + 3] = alpha as u8;
        }
    }
    unsafe {
        ImageList_Add(list, bitmap, null_mut());
        DeleteObject(bitmap);
    }
}

fn clipboard(window: Handle) -> Option<String> {
    unsafe {
        if OpenClipboard(window) == 0 {
            return None;
        }
        let handle = GetClipboardData(13);
        let result = if handle.is_null() {
            None
        } else {
            let length = (GlobalSize(handle) / 2).min(512 * 1024);
            let memory = GlobalLock(handle) as *const u16;
            if memory.is_null() {
                None
            } else {
                let units = std::slice::from_raw_parts(memory, length);
                let end = units
                    .iter()
                    .position(|unit| *unit == 0)
                    .unwrap_or(units.len());
                let text = String::from_utf16_lossy(&units[..end]);
                GlobalUnlock(handle);
                Some(text)
            }
        };
        CloseClipboard();
        result
    }
}

fn copy_clipboard(window: Handle, text: &str) {
    let text = wide(text);
    unsafe {
        let handle = GlobalAlloc(2, text.len() * 2);
        if handle.is_null() {
            return;
        }
        let memory = GlobalLock(handle);
        if memory.is_null() {
            GlobalFree(handle);
            return;
        }
        std::ptr::copy_nonoverlapping(text.as_ptr(), memory.cast(), text.len());
        GlobalUnlock(handle);
        if OpenClipboard(window) == 0 {
            GlobalFree(handle);
            return;
        }
        EmptyClipboard();
        if SetClipboardData(13, handle).is_null() {
            GlobalFree(handle);
        }
        CloseClipboard();
    }
}
