//! Native configuration, scheduler and grabber screens from the extracted dialog resources.
//! Resource geometry is retained; unavailable legacy integrations are visibly disabled.
#![allow(unsafe_op_in_unsafe_fn)]

use super::IdxApp;
mod grabber_filters;
use crate::grabber::filters::{FileRules, Files, PathRules, Paths, size_bytes};
use crate::model::{Queue, Settings, Status};
use chrono::{Datelike, Local, NaiveDate};
use eframe::egui;
use std::{
    ffi::c_void,
    ptr::{null, null_mut},
};

pub(super) type Handle = *mut c_void;
type Proc = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;
const USERDATA: i32 = -21;
const WM_INIT: u32 = 0x110;
const WM_COMMAND: u32 = 0x111;
const WM_NOTIFY: u32 = 0x4e;
const WM_TIMER: u32 = 0x113;
const WM_CLOSE: u32 = 0x10;
const TCM: u32 = 0x1300;
const LVM: u32 = 0x1000;
const TVM: u32 = 0x1100;
const TAB: i32 = 19500;
const DAYS: [i32; 7] = [1057, 1066, 1069, 1222, 1223, 1224, 1062];

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}
#[repr(C)]
struct Notify {
    hwnd: Handle,
    id: usize,
    code: u32,
}
// NMTVKEYDOWN is packed to one-byte alignment by commctrl.h.
#[repr(C, packed)]
struct TreeKeyDown {
    header: Notify,
    key: u16,
    flags: u32,
}
#[repr(C)]
struct TabItem {
    mask: u32,
    state: u32,
    state_mask: u32,
    text: *mut u16,
    text_max: i32,
    image: i32,
    param: isize,
}
#[repr(C)]
#[derive(Default)]
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
#[derive(Default)]
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
#[derive(Default)]
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
#[derive(Default)]
struct SystemTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}
#[repr(C)]
struct InitControls {
    size: u32,
    classes: u32,
}
#[repr(C)]
struct DrawItem {
    control_type: u32,
    control_id: u32,
    item_id: u32,
    action: u32,
    state: u32,
    window: Handle,
    dc: Handle,
    rect: Rect,
    data: usize,
}
#[repr(C)]
struct GdiStartupInput {
    version: u32,
    debug_callback: Handle,
    suppress_thread: i32,
    suppress_codecs: i32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn DialogBoxIndirectParamW(
        instance: Handle,
        template: *const c_void,
        parent: Handle,
        proc: Option<Proc>,
        param: isize,
    ) -> isize;
    fn CreateDialogIndirectParamW(
        instance: Handle,
        template: *const c_void,
        parent: Handle,
        proc: Option<Proc>,
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
    fn GetDlgItem(window: Handle, id: i32) -> Handle;
    fn GetParent(window: Handle) -> Handle;
    fn SetWindowLongPtrW(window: Handle, index: i32, value: isize) -> isize;
    fn GetWindowLongPtrW(window: Handle, index: i32) -> isize;
    fn SendMessageW(window: Handle, message: u32, w: usize, l: isize) -> isize;
    fn SetWindowTextW(window: Handle, text: *const u16) -> i32;
    fn GetWindowTextW(window: Handle, text: *mut u16, len: i32) -> i32;
    fn GetWindowTextLengthW(window: Handle) -> i32;
    fn ShowWindow(window: Handle, command: i32) -> i32;
    fn EnableWindow(window: Handle, enable: i32) -> i32;
    fn SetWindowPos(
        window: Handle,
        after: Handle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    fn MapDialogRect(window: Handle, rect: *mut Rect) -> i32;
    fn GetWindowRect(window: Handle, rect: *mut Rect) -> i32;
    fn GetClientRect(window: Handle, rect: *mut Rect) -> i32;
    fn MapWindowPoints(from: Handle, to: Handle, points: *mut Point, count: u32) -> i32;
    fn SetTimer(window: Handle, id: usize, ms: u32, proc: *const c_void) -> usize;
    fn KillTimer(window: Handle, id: usize) -> i32;
    fn EndDialog(window: Handle, result: isize) -> i32;
    fn MessageBoxW(window: Handle, text: *const u16, title: *const u16, flags: u32) -> i32;
    fn LoadImageW(
        instance: Handle,
        name: *const u16,
        kind: u32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> Handle;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> Handle;
    fn GetLastError() -> u32;
}
#[link(name = "comctl32")]
unsafe extern "system" {
    fn InitCommonControlsEx(value: *const InitControls) -> i32;
}
#[link(name = "gdiplus")]
unsafe extern "system" {
    fn GdiplusStartup(token: *mut usize, input: *const GdiStartupInput, output: Handle) -> i32;
    fn GdiplusShutdown(token: usize);
    fn GdipLoadImageFromStream(stream: Handle, image: *mut Handle) -> i32;
    fn GdipDisposeImage(image: Handle) -> i32;
    fn GdipCreateFromHDC(dc: Handle, graphics: *mut Handle) -> i32;
    fn GdipDeleteGraphics(graphics: Handle) -> i32;
    fn GdipDrawImageRectI(
        graphics: Handle,
        image: Handle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> i32;
}
#[link(name = "shlwapi")]
unsafe extern "system" {
    fn SHCreateMemStream(bytes: *const u8, length: u32) -> Handle;
}

/// Original IMAGE/384 JPEG, 128 × 260. The four wizard constructors load 0x180
/// through FUN_005155f0 and paint it into control 0x709 through FUN_005156f0.
/// SHA-256: 53b3b81211c9780b0923b347e5400313f573d8bb143922caae93687dd0ad5de5.
struct Illustration {
    token: usize,
    stream: Handle,
    image: Handle,
}
impl Illustration {
    unsafe fn load() -> Option<Self> {
        let mut picture = Self {
            token: 0,
            stream: null_mut(),
            image: null_mut(),
        };
        let input = GdiStartupInput {
            version: 1,
            debug_callback: null_mut(),
            suppress_thread: 0,
            suppress_codecs: 0,
        };
        if GdiplusStartup(&mut picture.token, &input, null_mut()) != 0 {
            return None;
        }
        let bytes = include_bytes!("../../assets/native/grabber_384.jpg");
        picture.stream = SHCreateMemStream(bytes.as_ptr(), bytes.len() as u32);
        if picture.stream.is_null()
            || GdipLoadImageFromStream(picture.stream, &mut picture.image) != 0
            || picture.image.is_null()
        {
            return None;
        }
        Some(picture)
    }
}
impl Drop for Illustration {
    fn drop(&mut self) {
        unsafe {
            if !self.image.is_null() {
                GdipDisposeImage(self.image);
            }
            // GDI+ requires its source IStream to remain alive until the image is disposed.
            if !self.stream.is_null() {
                let table = *(self.stream as *const *const *const c_void);
                let release: unsafe extern "system" fn(Handle) -> u32 =
                    std::mem::transmute(*table.add(2));
                release(self.stream);
            }
            if self.token != 0 {
                GdiplusShutdown(self.token);
            }
        }
    }
}
unsafe fn draw_illustration(item: &DrawItem) -> bool {
    let image = GetWindowLongPtrW(item.window, USERDATA) as Handle;
    if item.control_id != 1801 || image.is_null() {
        return false;
    }
    let mut graphics = null_mut();
    if GdipCreateFromHDC(item.dc, &mut graphics) != 0 || graphics.is_null() {
        return false;
    }
    let result = GdipDrawImageRectI(
        graphics,
        image,
        item.rect.left,
        item.rect.top,
        item.rect.right - item.rect.left,
        item.rect.bottom - item.rect.top,
    );
    GdipDeleteGraphics(graphics);
    result == 0
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
unsafe fn set(hwnd: Handle, id: i32, text: &str) {
    SetWindowTextW(
        if id == 0 { hwnd } else { GetDlgItem(hwnd, id) },
        wide(text).as_ptr(),
    );
}
unsafe fn get(hwnd: Handle, id: i32) -> String {
    let child = GetDlgItem(hwnd, id);
    let len = GetWindowTextLengthW(child).clamp(0, 65_536);
    let mut text = vec![0u16; len as usize + 1];
    let n = GetWindowTextW(child, text.as_mut_ptr(), len + 1);
    String::from_utf16_lossy(&text[..n.max(0) as usize])
}
unsafe fn enabled(hwnd: Handle, id: i32, on: bool) {
    EnableWindow(GetDlgItem(hwnd, id), on as i32);
}
unsafe fn check(hwnd: Handle, id: i32, on: bool) {
    SendMessageW(GetDlgItem(hwnd, id), 0xf1, on as usize, 0);
}
unsafe fn checked(hwnd: Handle, id: i32) -> bool {
    SendMessageW(GetDlgItem(hwnd, id), 0xf0, 0, 0) == 1
}
unsafe fn hide(hwnd: Handle, ids: &[i32]) {
    for &id in ids {
        ShowWindow(GetDlgItem(hwnd, id), 0);
    }
}
unsafe fn disable(hwnd: Handle, ids: &[i32]) {
    for &id in ids {
        enabled(hwnd, id, false);
    }
}
unsafe fn info(hwnd: Handle, text: &str) {
    MessageBoxW(
        hwnd,
        wide(text).as_ptr(),
        wide("IDX – Internet Downloader X").as_ptr(),
        0x40,
    );
}
unsafe fn rect(hwnd: Handle, x: i32, y: i32, w: i32, h: i32) -> Rect {
    let mut r = Rect {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    };
    MapDialogRect(hwnd, &mut r);
    r
}
unsafe fn control(
    parent: Handle,
    class: &str,
    title: &str,
    id: i32,
    style: u32,
    xywh: (i32, i32, i32, i32),
) -> Handle {
    let r = rect(parent, xywh.0, xywh.1, xywh.2, xywh.3);
    let child = CreateWindowExW(
        0,
        wide(class).as_ptr(),
        wide(title).as_ptr(),
        0x5000_0000 | style,
        r.left,
        r.top,
        r.right - r.left,
        r.bottom - r.top,
        parent,
        id as isize as Handle,
        GetModuleHandleW(null()),
        null_mut(),
    );
    SendMessageW(child, 0x30, SendMessageW(parent, 0x31, 0, 0) as usize, 1);
    child
}
unsafe fn center(hwnd: Handle, owner: Handle) {
    if owner.is_null() {
        return;
    }
    let (mut a, mut b) = (Rect::default(), Rect::default());
    GetWindowRect(owner, &mut a);
    GetWindowRect(hwnd, &mut b);
    SetWindowPos(
        hwnd,
        null_mut(),
        a.left + (a.right - a.left - b.right + b.left) / 2,
        a.top + (a.bottom - a.top - b.bottom + b.top) / 2,
        0,
        0,
        0x15,
    );
}
unsafe fn icon(hwnd: Handle) {
    for (size, kind) in [(16, 0), (32, 1)] {
        let icon = LoadImageW(
            GetModuleHandleW(null()),
            101usize as *const u16,
            1,
            size,
            size,
            0x8000,
        );
        if !icon.is_null() {
            SendMessageW(hwnd, 0x80, kind, icon as isize);
        }
    }
}
unsafe fn combo(hwnd: Handle, id: i32, items: &[&str], index: usize) {
    let h = GetDlgItem(hwnd, id);
    SendMessageW(h, 0x14b, 0, 0);
    for item in items {
        SendMessageW(h, 0x143, 0, wide(item).as_ptr() as isize);
    }
    SendMessageW(h, 0x14e, index, 0);
}
unsafe fn selected_combo(hwnd: Handle, id: i32) -> usize {
    SendMessageW(GetDlgItem(hwnd, id), 0x147, 0, 0).max(0) as usize
}
unsafe fn number(hwnd: Handle, id: i32, fallback: u64, min: u64, max: u64) -> u64 {
    get(hwnd, id)
        .trim()
        .parse::<u64>()
        .unwrap_or(fallback)
        .clamp(min, max)
}
unsafe fn columns(list: Handle, values: &[(&str, i32)]) {
    for (index, (name, width)) in values.iter().enumerate() {
        let mut name = wide(name);
        let c = ListColumn {
            mask: 6,
            width: *width,
            text: name.as_mut_ptr(),
            ..Default::default()
        };
        SendMessageW(list, LVM + 97, index, (&c as *const ListColumn) as isize);
    }
    SendMessageW(list, LVM + 54, 0, 0x10020);
}
unsafe fn row(list: Handle, index: usize, text: &str, param: isize) {
    let mut text = wide(text);
    let item = ListItem {
        mask: 5,
        item: index as i32,
        text: text.as_mut_ptr(),
        param,
        ..Default::default()
    };
    SendMessageW(list, LVM + 77, 0, (&item as *const ListItem) as isize);
}
unsafe fn cell(list: Handle, index: usize, subitem: i32, text: &str) {
    let mut text = wide(text);
    let item = ListItem {
        subitem,
        text: text.as_mut_ptr(),
        ..Default::default()
    };
    SendMessageW(list, LVM + 116, index, (&item as *const ListItem) as isize);
}
unsafe fn select_row(list: Handle, index: usize) {
    let item = ListItem {
        state: 3,
        state_mask: 3,
        ..Default::default()
    };
    SendMessageW(list, LVM + 43, index, (&item as *const ListItem) as isize);
}
unsafe fn selected_row(list: Handle) -> Option<usize> {
    let i = SendMessageW(list, LVM + 12, usize::MAX, 2);
    (i >= 0).then_some(i as usize)
}
unsafe fn tree_item(tree: Handle, text: &str, param: isize) -> Handle {
    let mut text = wide(text);
    let insert = TreeInsert {
        parent: (-65536isize) as Handle,
        after: (-65534isize) as Handle,
        item: TreeItem {
            mask: 5,
            text: text.as_mut_ptr(),
            param,
            ..Default::default()
        },
    };
    SendMessageW(tree, TVM + 50, 0, (&insert as *const TreeInsert) as isize) as Handle
}
unsafe fn tree_selection(tree: Handle) -> Option<usize> {
    let h = SendMessageW(tree, TVM + 10, 9, 0) as Handle;
    if h.is_null() {
        return None;
    }
    let mut item = TreeItem {
        mask: 4,
        item: h,
        ..Default::default()
    };
    (SendMessageW(tree, TVM + 62, 0, (&mut item as *mut TreeItem) as isize) != 0)
        .then_some(item.param.max(0) as usize)
}
unsafe fn time_set(hwnd: Handle, id: i32, hour: u32, minute: u32) {
    let t = SystemTime {
        year: 2026,
        month: 1,
        day: 1,
        hour: hour as u16,
        minute: minute as u16,
        ..Default::default()
    };
    let child = GetDlgItem(hwnd, id);
    SendMessageW(child, 0x1032, 0, wide("HH:mm").as_ptr() as isize);
    SendMessageW(child, 0x1002, 0, (&t as *const SystemTime) as isize);
}
unsafe fn time_get(hwnd: Handle, id: i32) -> (u32, u32) {
    let mut t = SystemTime::default();
    SendMessageW(
        GetDlgItem(hwnd, id),
        0x1001,
        0,
        (&mut t as *mut SystemTime) as isize,
    );
    (u32::from(t.hour).min(23), u32::from(t.minute).min(59))
}
unsafe fn date_set(hwnd: Handle, id: i32, date: NaiveDate) {
    let value = SystemTime {
        year: date.year() as u16,
        month: date.month() as u16,
        day: date.day() as u16,
        ..Default::default()
    };
    SendMessageW(
        GetDlgItem(hwnd, id),
        0x1002,
        0,
        (&value as *const SystemTime) as isize,
    );
}
unsafe fn date_get(hwnd: Handle, id: i32) -> Option<NaiveDate> {
    let mut value = SystemTime::default();
    let result = SendMessageW(
        GetDlgItem(hwnd, id),
        0x1001,
        0,
        (&mut value as *mut SystemTime) as isize,
    );
    (result == 0)
        .then(|| {
            NaiveDate::from_ymd_opt(
                i32::from(value.year),
                u32::from(value.month),
                u32::from(value.day),
            )
        })
        .flatten()
}

fn child_template(id: u32) -> Vec<u32> {
    let mut data = super::native_resources::dialog(id);
    if data.is_empty() {
        return data;
    }
    let bytes =
        unsafe { std::slice::from_raw_parts_mut(data.as_mut_ptr().cast::<u8>(), data.len() * 4) };
    let offset = if bytes.get(2..4) == Some(&[255, 255]) {
        12
    } else {
        0
    };
    let mut style = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
    // Property-sheet resources can begin disabled; the native host activates each page.
    style = (style & !0x88cf_0000) | 0x5000_0400;
    bytes[offset..offset + 4].copy_from_slice(&style.to_le_bytes());
    data
}
fn host_template() -> Vec<u32> {
    let mut b = vec![];
    b.extend_from_slice(&0x80c8_00c0u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    for n in [0u16, 0, 0, 282, 310, 0, 0] {
        b.extend_from_slice(&n.to_le_bytes());
    }
    for c in wide("IDX Configuration") {
        b.extend_from_slice(&c.to_le_bytes());
    }
    b.extend_from_slice(&8u16.to_le_bytes());
    for c in wide("Tahoma") {
        b.extend_from_slice(&c.to_le_bytes());
    }
    while b.len() % 4 != 0 {
        b.push(0);
    }
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Options,
    Scheduler,
    Grabber,
    Results,
}
struct State {
    app: *mut IdxApp,
    kind: Kind,
    owner: Handle,
    hwnd: Handle,
    pages: Vec<(u32, Handle)>,
    active: usize,
    busy: bool,
    context: egui::Context,
    settings: Settings,
    options_category: usize,
    queue: usize,
    queue_rows: Vec<u64>,
    queue_snapshot: Vec<(u64, String, Status)>,
    result_rows: Vec<usize>,
    scanning: bool,
    wizard_step: usize,
    advanced: [bool; 4],
    folder: String,
    category: usize,
    add_results: bool,
    illustration: Option<Illustration>,
}
impl State {
    fn new(app: &mut IdxApp, owner: Handle, kind: Kind) -> Self {
        Self {
            settings: app.lib.settings.clone(),
            options_category: 0,
            queue: app.queue_index,
            app,
            kind,
            owner,
            hwnd: null_mut(),
            pages: vec![],
            active: 0,
            busy: false,
            context: egui::Context::default(),
            queue_rows: vec![],
            queue_snapshot: vec![],
            result_rows: vec![],
            scanning: false,
            wizard_step: 0,
            advanced: [false; 4],
            folder: String::new(),
            category: 0,
            add_results: false,
            illustration: None,
        }
    }
    fn page(&self, id: u32) -> Handle {
        self.pages
            .iter()
            .find(|p| p.0 == id)
            .map(|p| p.1)
            .unwrap_or(null_mut())
    }
}

pub(super) fn options(owner: Handle, app: &mut IdxApp) {
    let mut state = State::new(app, owner, Kind::Options);
    unsafe {
        run(&mut state, host_template());
    }
    app.options = false;
}

struct BrowserSetupState {
    app: *mut IdxApp,
    context: egui::Context,
    busy: bool,
}

pub(super) fn browser_setup(owner: Handle, app: &mut IdxApp) {
    let data = app
        .library_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_owned();
    if let Err(error) = crate::browser_setup::mark_offered(&data) {
        app.message = Some(error);
    }
    let mut state = BrowserSetupState {
        app,
        context: egui::Context::default(),
        busy: true,
    };
    let template = super::native_resources::top_level(381);
    unsafe {
        if DialogBoxIndirectParamW(
            GetModuleHandleW(null()),
            template.as_ptr().cast(),
            owner,
            Some(browser_setup_proc),
            (&mut state as *mut BrowserSetupState) as isize,
        ) == -1
        {
            (*state.app).message = Some("Could not open browser connection setup. Open Options > General > Extension setup to try again.".into());
        }
    }
}

unsafe extern "system" fn browser_setup_proc(hwnd: Handle, msg: u32, w: usize, l: isize) -> isize {
    if msg == WM_INIT {
        SetWindowLongPtrW(hwnd, USERDATA, l);
        let state = &mut *(l as *mut BrowserSetupState);
        set(hwnd, 0, "IDX browser connection");
        set(hwnd, 1799, "Connect Chrome or Edge to IDX");
        hide(hwnd, &[1797]);
        control(
            hwnd,
            "Static",
            "1. Open your browser's Extensions page: chrome://extensions or edge://extensions.\r\n2. Turn on Developer mode. Choose Load unpacked.\r\n3. Choose the extension folder opened by the button below.\r\n4. IDX connects automatically. Right-click a file link and choose Download with IDX.",
            19000,
            0,
            (8, 31, 462, 88),
        );
        control(
            hwnd,
            "Button",
            "Open extension folder",
            19001,
            0x10000,
            (8, 123, 112, 18),
        );
        control(
            hwnd,
            "Button",
            "Reconnect browser",
            19003,
            0x10000,
            (132, 123, 112, 18),
        );
        set(
            hwnd,
            1798,
            "The desktop connection is configured automatically when IDX opens. You can skip extension installation and use IDX now. Return here from Options > General > Extension setup.",
        );
        set(hwnd, 1, "Skip");
        icon(hwnd);
        center(hwnd, GetParent(hwnd));
        SetTimer(hwnd, 1, 150, null());
        state.busy = false;
        return 1;
    }
    let ptr = GetWindowLongPtrW(hwnd, USERDATA) as *mut BrowserSetupState;
    if ptr.is_null() || (*ptr).busy {
        return 0;
    }
    (*ptr).busy = true;
    let state = &mut *ptr;
    match msg {
        WM_COMMAND if w >> 16 == 0 => match w & 0xffff {
            1 | 2 => {
                EndDialog(hwnd, 0);
            }
            19001 => {
                match std::env::current_exe()
                    .ok()
                    .and_then(|p| crate::browser_setup::extension_folder(&p))
                {
                    Some(folder) => super::open_path(&folder),
                    None => info(
                        hwnd,
                        "The extension folder is missing. Reinstall IDX or extract the complete portable ZIP.",
                    ),
                }
            }
            19003 => {
                let data = (&*state.app)
                    .library_path
                    .parent()
                    .unwrap_or(std::path::Path::new("."));
                let result = std::env::current_exe()
                    .map_err(|e| e.to_string())
                    .and_then(|exe| crate::browser_setup::connect_bundled(data, &exe));
                match result {
                    Ok(()) => {
                        set(
                            hwnd,
                            1798,
                            "Connected. In Chrome or Edge, right-click a file link and choose Download with IDX. The extension's Check button can confirm the connection. Automatic download capture remains optional.",
                        );
                        set(hwnd, 1, "Close");
                    }
                    Err(error) => info(hwnd, &error),
                }
            }
            _ => {}
        },
        WM_TIMER => {
            (*state.app).tick(&state.context);
            (*state.app).sync_native_progress();
        }
        WM_CLOSE => {
            EndDialog(hwnd, 0);
        }
        2 => {
            KillTimer(hwnd, 1);
        }
        _ => {
            state.busy = false;
            return 0;
        }
    }
    state.busy = false;
    1
}
pub(super) fn scheduler(owner: Handle, app: &mut IdxApp) {
    let mut state = State::new(app, owner, Kind::Scheduler);
    unsafe {
        run(&mut state, super::native_resources::top_level(308));
    }
    app.scheduler = false;
}
pub(super) fn grabber(owner: Handle, app: &mut IdxApp) {
    let mut state = State::new(app, owner, Kind::Grabber);
    unsafe {
        run(&mut state, super::native_resources::top_level(239));
        if state.add_results {
            state.kind = Kind::Results;
            state.pages.clear();
            state.add_results = false;
            run(&mut state, stored_results_template());
        }
    }
    app.grabber = false;
}
fn stored_results_template() -> Vec<u32> {
    let mut template = super::native_resources::top_level(241);
    if template.is_empty() {
        return template;
    }
    let extended = template[0] == 0xffff0001;
    let bytes = unsafe {
        std::slice::from_raw_parts_mut(template.as_mut_ptr().cast::<u8>(), template.len() * 4)
    };
    let class: Vec<u8> = "SysListView32"
        .encode_utf16()
        .chain(Some(0))
        .flat_map(u16::to_le_bytes)
        .collect();
    if let Some(class_offset) = bytes.windows(class.len()).position(|part| part == class) {
        let header_length = if extended { 24 } else { 18 };
        if let Some(header) = class_offset.checked_sub(header_length) {
            let control_id = if extended {
                u32::from_le_bytes(bytes[header + 20..header + 24].try_into().unwrap())
            } else {
                u32::from(u16::from_le_bytes(
                    bytes[header + 16..header + 18].try_into().unwrap(),
                ))
            };
            if control_id == 1023 {
                let style_offset = header + if extended { 8 } else { 0 };
                let style =
                    u32::from_le_bytes(bytes[style_offset..style_offset + 4].try_into().unwrap());
                // Resource241 used an IDM virtual-list callback for text (LVS_OWNERDATA).
                // IDX inserts its own rows, so select stored-item mode before creation.
                bytes[style_offset..style_offset + 4]
                    .copy_from_slice(&(style & !0x1000).to_le_bytes());
            }
        }
    }
    template
}
unsafe fn run(state: &mut State, template: Vec<u32>) {
    if template.is_empty() {
        return;
    }
    InitCommonControlsEx(&InitControls {
        size: std::mem::size_of::<InitControls>() as u32,
        classes: 0xffff,
    });
    if DialogBoxIndirectParamW(
        GetModuleHandleW(null()),
        template.as_ptr().cast(),
        state.owner,
        Some(proc),
        state as *mut State as isize,
    ) == -1
    {
        (*state.app).message = Some(format!(
            "Could not open a native dialog (Windows error {}).",
            GetLastError()
        ));
    }
}
unsafe fn add_page(state: &mut State, id: u32, x: i32, y: i32) -> Handle {
    let template = child_template(id);
    if template.is_empty() {
        return null_mut();
    }
    let h = CreateDialogIndirectParamW(
        GetModuleHandleW(null()),
        template.as_ptr().cast(),
        state.hwnd,
        Some(page_proc),
        0,
    );
    if !h.is_null() {
        let r = rect(state.hwnd, x, y, 0, 0);
        SetWindowPos(h, null_mut(), r.left, r.top, 0, 0, 0x15);
        ShowWindow(h, 0);
        state.pages.push((id, h));
    }
    h
}
unsafe fn tabs(state: &mut State, labels: &[&str], xywh: (i32, i32, i32, i32)) {
    let tab = control(
        state.hwnd,
        "SysTabControl32",
        "",
        TAB,
        0x10000
            | if state.kind == Kind::Options {
                0x200
            } else {
                0
            },
        xywh,
    );
    for (index, label) in labels.iter().enumerate() {
        let mut label = wide(label);
        let item = TabItem {
            mask: 1,
            state: 0,
            state_mask: 0,
            text: label.as_mut_ptr(),
            text_max: 0,
            image: 0,
            param: 0,
        };
        SendMessageW(tab, TCM + 62, index, (&item as *const TabItem) as isize);
    }
}
unsafe fn show_page(state: &mut State, index: usize) {
    state.active = index;
    for (i, (_, page)) in state.pages.iter().enumerate() {
        ShowWindow(*page, if i == index { 5 } else { 0 });
    }
    if state.kind == Kind::Grabber {
        layout_grabber(state);
    }
}
unsafe fn fit_option_tabs(state: &State) {
    let tab = GetDlgItem(state.hwnd, TAB);
    let (mut area, mut tab_window, mut root) = (Rect::default(), Rect::default(), Rect::default());
    GetClientRect(tab, &mut area);
    SendMessageW(tab, TCM + 40, 0, (&mut area as *mut Rect) as isize);
    let page_height = state
        .pages
        .iter()
        .map(|(_, page)| {
            let mut r = Rect::default();
            GetWindowRect(*page, &mut r);
            r.bottom - r.top
        })
        .max()
        .unwrap_or(0);
    let extra = (page_height - (area.bottom - area.top)).max(0);
    let page_width = state
        .pages
        .iter()
        .map(|(_, page)| {
            let mut r = Rect::default();
            GetWindowRect(*page, &mut r);
            r.right - r.left
        })
        .max()
        .unwrap_or(0);
    let extra_width = (page_width - (area.right - area.left)).max(0);
    if extra > 0 || extra_width > 0 {
        GetWindowRect(tab, &mut tab_window);
        SetWindowPos(
            tab,
            null_mut(),
            0,
            0,
            tab_window.right - tab_window.left + extra_width,
            tab_window.bottom - tab_window.top + extra,
            0x16,
        );
        GetWindowRect(state.hwnd, &mut root);
        SetWindowPos(
            state.hwnd,
            null_mut(),
            0,
            0,
            root.right - root.left + extra_width,
            root.bottom - root.top + extra,
            0x16,
        );
        for id in [1, 2, 1089] {
            let h = GetDlgItem(state.hwnd, id);
            let mut r = Rect::default();
            GetWindowRect(h, &mut r);
            MapWindowPoints(null_mut(), state.hwnd, (&mut r as *mut Rect).cast(), 2);
            SetWindowPos(
                h,
                null_mut(),
                r.left + extra_width,
                r.top + extra,
                0,
                0,
                0x15,
            );
        }
    }
    GetClientRect(tab, &mut area);
    SendMessageW(tab, TCM + 40, 0, (&mut area as *mut Rect) as isize);
    MapWindowPoints(tab, state.hwnd, (&mut area as *mut Rect).cast(), 2);
    for (_, page) in &state.pages {
        SetWindowPos(*page, null_mut(), area.left, area.top, 0, 0, 0x15);
    }
    center(state.hwnd, state.owner);
}
unsafe extern "system" fn page_proc(hwnd: Handle, message: u32, w: usize, l: isize) -> isize {
    match message {
        0x2b if l != 0 => draw_illustration(&*(l as *const DrawItem)) as isize,
        WM_COMMAND | WM_NOTIFY => {
            SendMessageW(GetParent(hwnd), message, w, l);
            1
        }
        _ => 0,
    }
}
unsafe extern "system" fn proc(hwnd: Handle, message: u32, w: usize, l: isize) -> isize {
    if message == WM_INIT {
        SetWindowLongPtrW(hwnd, USERDATA, l);
        let state = &mut *(l as *mut State);
        state.hwnd = hwnd;
        state.busy = true;
        icon(hwnd);
        center(hwnd, state.owner);
        match state.kind {
            Kind::Options => init_options(state),
            Kind::Scheduler => init_scheduler(state),
            Kind::Grabber => init_grabber(state),
            Kind::Results => init_results(state),
        }
        SetTimer(hwnd, 1, 150, null());
        state.busy = false;
        return 1;
    }
    let ptr = GetWindowLongPtrW(hwnd, USERDATA) as *mut State;
    if ptr.is_null() || (*ptr).busy {
        return 0;
    }
    (*ptr).busy = true;
    let state = &mut *ptr;
    let result = match message {
        WM_COMMAND => {
            let id = (w & 0xffff) as i32;
            let code = (w >> 16) as u16;
            if code == 0 || code == 1 {
                match state.kind {
                    Kind::Options => options_command(state, id),
                    Kind::Scheduler => scheduler_command(state, id),
                    Kind::Grabber => grabber_command(state, id),
                    Kind::Results => results_command(state, id),
                }
            }
            1
        }
        WM_NOTIFY => {
            let n = &*(l as *const Notify);
            if n.id == TAB as usize && n.code == (-551i32) as u32 {
                let index = SendMessageW(GetDlgItem(hwnd, TAB), TCM + 11, 0, 0).max(0) as usize;
                show_page(state, index);
            }
            if state.kind == Kind::Scheduler && n.id == 1682 && n.code == (-451i32) as u32 {
                if let Some(index) = tree_selection(GetDlgItem(hwnd, 1682)) {
                    if index != state.queue {
                        save_queue(state);
                        state.queue = index;
                        load_queue(state);
                    }
                }
            }
            if state.kind == Kind::Scheduler
                && n.id == 1682
                && (n.code == (-3i32) as u32
                    || (n.code == (-412i32) as u32 && (*(l as *const TreeKeyDown)).key == 0x71))
            {
                rename_queue(state);
            }
            1
        }
        WM_TIMER => {
            (*state.app).tick(&state.context);
            (*state.app).sync_native_progress();
            if state.kind == Kind::Scheduler && state.active == 1 {
                refresh_files(state, None);
            }
            if state.kind == Kind::Grabber && state.scanning && (*state.app).grab_rx.is_none() {
                state.scanning = false;
                if (&(*state.app).grab_error).is_empty() {
                    state.add_results = true;
                    EndDialog(hwnd, 1);
                } else {
                    info(hwnd, &(*state.app).grab_error);
                    set(hwnd, 1430, "Scan site");
                    enabled(hwnd, 1430, true);
                    enabled(hwnd, 1429, true);
                    enabled(state.page(240), 1405, true);
                    enabled(state.page(240), 1421, true);
                }
            }
            if state.kind == Kind::Grabber && state.scanning {
                set(state.page(240), 1471, &(*state.app).grab_status);
            }
            1
        }
        WM_CLOSE => {
            close(state);
            1
        }
        2 => {
            KillTimer(hwnd, 1);
            0
        }
        _ => 0,
    };
    (*ptr).busy = false;
    result
}
unsafe fn close(state: &mut State) {
    if state.kind == Kind::Grabber {
        (*state.app).cancel_grabber();
    }
    if state.kind == Kind::Scheduler {
        save_queue(state);
        (*state.app).queue_index = state.queue;
        (*state.app).save();
    }
    EndDialog(state.hwnd, 0);
}

unsafe fn init_options(state: &mut State) {
    tabs(
        state,
        &[
            "General",
            "File types",
            "Save to",
            "Downloads",
            "Connection",
            "Proxy / Socks",
            "Sites Logins",
            "Dial Up / VPN",
            "Sounds",
        ],
        (5, 5, 272, 276),
    );
    for id in [139, 294, 143, 291, 141, 140, 145, 185, 195] {
        add_page(state, id, 6, 24);
    }
    control(state.hwnd, "Button", "OK", 1, 0x10001, (112, 289, 50, 14));
    control(
        state.hwnd,
        "Button",
        "Cancel",
        2,
        0x10000,
        (167, 289, 50, 14),
    );
    control(
        state.hwnd,
        "Button",
        "Apply",
        1089,
        0x10000,
        (222, 289, 50, 14),
    );
    let general = state.page(139);
    disable(general, &[1056, 1061, 1650, 1367, 1688, 1721, 1866]);
    set(general, 1865, "Browser integration uses the IDX extension");
    set(general, 1633, "Extension setup...");
    let list = GetDlgItem(general, 1351);
    columns(list, &[("Browser", 170), ("Integration", 170)]);
    for (i, name) in [
        "Google Chrome / Chromium",
        "Microsoft Edge",
        "Mozilla Firefox",
    ]
    .iter()
    .enumerate()
    {
        row(list, i, name, 0);
        cell(list, i, 1, "Optional extension");
    }
    let types = state.page(294);
    set(
        types,
        1270,
        "Automatic file capture is configured in the browser extension.",
    );
    set(types, 1071, "ZIP RAR 7Z EXE MSI PDF MP3 MP4 MKV ISO");
    disable(types, &[1071, 1031, 1077, 1629]);
    set(types, 1656, "Extension setup...");
    let save = state.page(143);
    disable(save, &[1795, 1217, 1054]);
    load_options_category(state);
    set(save, 1217, "Alongside each destination file");
    set(
        save,
        1625,
        "Download parts are saved alongside the destination so a completed file can be finalized safely without overwriting an existing file.",
    );
    let down = state.page(291);
    disable(
        down,
        &[1688, 1059, 1862, 1236, 1623, 1739, 1738, 1615, 1443, 1841],
    );
    check(down, 1059, true);
    check(down, 1236, true);
    combo(
        down,
        1443,
        &["Keep both downloads with distinct file names"],
        0,
    );
    set(down, 1840, &state.settings.user_agent);
    set(down, 1838, "External virus checking is not configured");
    let conn = state.page(141);
    combo(
        conn,
        1206,
        &[
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
        ],
        state.settings.connections.saturating_sub(1).min(15),
    );
    disable(conn, &[1344, 1345, 1346, 1023, 1268, 1842]);
    set(conn, 1254, "Speed limit");
    set(conn, 1266, "Limit each download to");
    set(conn, 1264, "KB / sec");
    set(conn, 1267, "Retries");
    set(conn, 1265, "per file (0–10)");
    check(conn, 1254, state.settings.speed_limit_kib > 0);
    set(conn, 1046, &state.settings.speed_limit_kib.to_string());
    set(conn, 1074, &state.settings.retries.to_string());
    set(conn, 1842, "Secure HTTPS downloads");
    check(conn, 1842, true);
    let proxy = state.page(140);
    disable(
        proxy,
        &[1844, 1826, 1796, 1660, 1782, 1783, 1784, 1376, 1116],
    );
    check(proxy, 1782, true);
    check(proxy, 1783, true);
    set(proxy, 1777, "Proxy URL help...");
    set(
        proxy,
        1377,
        "One proxy is used for HTTP and HTTPS downloads.",
    );
    set(proxy, 1103, &state.settings.proxy);
    check(
        proxy,
        if state.settings.proxy.is_empty() {
            1825
        } else {
            1823
        },
        true,
    );
    if let Ok(url) = url::Url::parse(&state.settings.proxy) {
        set(
            proxy,
            1103,
            &format!("{}://{}", url.scheme(), url.host_str().unwrap_or("")),
        );
        set(
            proxy,
            1104,
            &url.port().map(|v| v.to_string()).unwrap_or_default(),
        );
        set(proxy, 1105, &decode_component(url.username()));
        set(proxy, 1106, &decode_component(url.password().unwrap_or("")));
    }
    let sites = state.page(145);
    disable(sites, &[1024, 1087, 1196, 1050]);
    set(sites, 1530, "Website credentials are entered per download");
    control(
        sites,
        "Static",
        "Use More >> in download information to enter website credentials. Saved site-wide logins are not implemented.",
        19520,
        0,
        (7, 230, 255, 24),
    );
    let dial = state.page(185);
    disable(dial, &[1058, 1208, 1037, 1045, 1237, 1166, 1212, 1249]);
    set(dial, 1526, "Dial Up / VPN control is not implemented");
    control(
        dial,
        "Static",
        "Connect a VPN through Windows before starting a download. IDX uses the existing network connection.",
        19521,
        0,
        (7, 228, 254, 24),
    );
    let sound = state.page(195);
    disable(sound, &[1172, 1091]);
    set(sound, 1320, "Sound customization is not implemented in IDX");
    columns(GetDlgItem(sound, 1028), &[("Event", 180), ("Sound", 120)]);
    for (i, name) in ["Download complete", "Download failed", "Queue finished"]
        .iter()
        .enumerate()
    {
        row(GetDlgItem(sound, 1028), i, name, 0);
        cell(GetDlgItem(sound, 1028), i, 1, "Not configured");
    }
    fit_option_tabs(state);
    show_page(state, 0);
}
fn decode_component(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let high = (bytes[i + 1] as char).to_digit(16);
            let low = (bytes[i + 2] as char).to_digit(16);
            if let (Some(high), Some(low)) = (high, low) {
                out.push((high * 16 + low) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
// COptSave 0x50d6c0 saves the selected category's directory and remember flag.
unsafe fn save_options_category(state: &mut State) -> bool {
    let page = state.page(143);
    let folder = std::path::PathBuf::from(get(page, 1032).trim());
    if !folder.is_absolute() || folder.is_file() {
        info(state.hwnd, "Choose an absolute save directory, not a file.");
        return false;
    }
    if let Some(i) = state.options_category.checked_sub(1) {
        let Some(mut category) = state.settings.categories.get(i).cloned() else {
            return false;
        };
        if folder != state.settings.category_folder(&category.name) {
            category.folder = Some(folder);
        }
        category.extensions = get(page, 1038);
        category.remember_last_path = checked(page, 1190);
        match crate::categories::validate(category, &state.settings.categories, Some(i)) {
            Ok(category) => state.settings.categories[i] = category,
            Err(error) => {
                info(state.hwnd, &error);
                return false;
            }
        }
    } else {
        state.settings.download_dir = folder;
        state.settings.remember_last_path = checked(page, 1190);
    }
    true
}

unsafe fn load_options_category(state: &mut State) {
    let page = state.page(143);
    let names: Vec<_> = std::iter::once("General")
        .chain(state.settings.categories.iter().map(|c| c.name.as_str()))
        .collect();
    combo(page, 1282, &names, state.options_category);
    let category = state
        .options_category
        .checked_sub(1)
        .and_then(|i| state.settings.categories.get(i));
    let name = category.map_or("General", |c| c.name.as_str());
    set(
        page,
        1624,
        &format!("Automatically put in \"{name}\" category following file types"),
    );
    set(
        page,
        1136,
        &format!("Default download directory for \"{name}\" category"),
    );
    set(
        page,
        1190,
        &format!("Change folder for \"{name}\" category on last selected"),
    );
    set(page, 1038, category.map_or("", |c| c.extensions.as_str()));
    check(
        page,
        1190,
        category.map_or(state.settings.remember_last_path, |c| c.remember_last_path),
    );
    let folder = category.map_or_else(
        || state.settings.download_dir.clone(),
        |c| state.settings.category_folder(&c.name),
    );
    set(page, 1032, &folder.to_string_lossy());
    enabled(page, 1038, category.is_some());
    enabled(page, 1346, category.is_some());
}

struct CategoryEditor {
    app: *mut IdxApp,
    context: egui::Context,
    category: crate::categories::Category,
    categories: Vec<crate::categories::Category>,
    editing: Option<usize>,
    result: Option<crate::categories::Category>,
    busy: bool,
}

/// CAddTreeItem constructors 0x4059a0 / 0x4062b0 both use original dialog 204.
pub(super) unsafe fn edit_category(
    owner: Handle,
    app: *mut IdxApp,
    category: crate::categories::Category,
    categories: &[crate::categories::Category],
    editing: Option<usize>,
) -> Result<Option<crate::categories::Category>, String> {
    let mut state = CategoryEditor {
        app,
        context: egui::Context::default(),
        category,
        categories: categories.to_vec(),
        editing,
        result: None,
        busy: true,
    };
    let template = super::native_resources::top_level(204);
    let result = DialogBoxIndirectParamW(
        GetModuleHandleW(null()),
        template.as_ptr().cast(),
        owner,
        Some(category_proc),
        (&mut state as *mut CategoryEditor) as isize,
    );
    if result == -1 {
        return Err("Could not open the category editor.".into());
    }
    Ok(state.result)
}

unsafe extern "system" fn category_proc(hwnd: Handle, msg: u32, w: usize, l: isize) -> isize {
    if msg == WM_INIT {
        SetWindowLongPtrW(hwnd, USERDATA, l);
        let s = &mut *(l as *mut CategoryEditor);
        set(hwnd, 1075, &s.category.name);
        set(hwnd, 1038, &s.category.extensions);
        set(hwnd, 1792, &s.category.sites);
        set(
            hwnd,
            1047,
            &s.category
                .folder
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
        );
        check(hwnd, 1790, s.category.sites_only);
        check(hwnd, 1239, s.category.remember_last_path);
        enabled(hwnd, 1792, s.category.sites_only);
        SendMessageW(GetDlgItem(hwnd, 1075), 0xc5, 128, 0);
        icon(hwnd);
        center(hwnd, GetParent(hwnd));
        SetTimer(hwnd, 1, 150, null());
        s.busy = false;
        return 1;
    }
    let ptr = GetWindowLongPtrW(hwnd, USERDATA) as *mut CategoryEditor;
    if ptr.is_null() || (*ptr).busy {
        return 0;
    }
    (*ptr).busy = true;
    let s = &mut *ptr;
    match msg {
        WM_COMMAND => match w & 0xffff {
            1790 => enabled(hwnd, 1792, checked(hwnd, 1790)),
            1174 => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(get(hwnd, 1047))
                    .pick_folder()
                {
                    set(hwnd, 1047, &path.to_string_lossy());
                }
            }
            1 => {
                let mut draft = s.category.clone();
                draft.name = get(hwnd, 1075);
                draft.extensions = get(hwnd, 1038);
                draft.sites = get(hwnd, 1792);
                draft.sites_only = checked(hwnd, 1790);
                draft.remember_last_path = checked(hwnd, 1239);
                draft.folder = Some(std::path::PathBuf::from(get(hwnd, 1047).trim()));
                if draft.folder.as_ref().is_some_and(|p| p.is_file()) {
                    info(hwnd, "Choose a save directory, not a file.");
                } else {
                    match crate::categories::validate(draft, &s.categories, s.editing) {
                        Ok(category) => {
                            s.result = Some(category);
                            EndDialog(hwnd, 1);
                        }
                        Err(error) => info(hwnd, &error),
                    }
                }
            }
            2 => {
                EndDialog(hwnd, 0);
            }
            _ => {}
        },
        WM_TIMER => {
            (*s.app).tick(&s.context);
            (*s.app).sync_native_progress();
        }
        WM_CLOSE => {
            EndDialog(hwnd, 0);
        }
        2 => {
            KillTimer(hwnd, 1);
        }
        _ => {
            s.busy = false;
            return 0;
        }
    }
    s.busy = false;
    1
}

unsafe fn options_command(state: &mut State, id: i32) {
    match id {
        1282 => {
            let selected = selected_combo(state.page(143), 1282);
            if save_options_category(state) {
                state.options_category = selected;
            }
            load_options_category(state);
        }
        1344 | 1346 => {
            if !save_options_category(state) {
                return;
            }
            let editing = (id == 1346)
                .then(|| state.options_category.checked_sub(1))
                .flatten();
            if id == 1346 && editing.is_none() {
                return;
            }
            let category = editing
                .and_then(|i| state.settings.categories.get(i).cloned())
                .unwrap_or_else(|| crate::categories::Category {
                    folder: Some(state.settings.download_dir.clone()),
                    ..Default::default()
                });
            let mut category = category;
            if category.folder.is_none() {
                category.folder = Some(state.settings.category_folder(&category.name));
            }
            match edit_category(
                state.hwnd,
                state.app,
                category,
                &state.settings.categories,
                editing,
            ) {
                Ok(Some(category)) => {
                    let i = editing.unwrap_or(state.settings.categories.len());
                    if i == state.settings.categories.len() {
                        state.settings.categories.push(category);
                    } else {
                        state.settings.categories[i] = category;
                    }
                    state.options_category = i + 1;
                    load_options_category(state);
                }
                Err(error) => info(state.hwnd, &error),
                _ => {}
            }
        }
        2 => {
            EndDialog(state.hwnd, 0);
        }
        1 | 1089 => {
            if save_options(state) && id == 1 {
                EndDialog(state.hwnd, 1);
            }
        }
        1049 => {
            let page = state.page(143);
            if let Some(path) = rfd::FileDialog::new()
                .set_directory(get(page, 1032))
                .pick_folder()
            {
                set(page, 1032, &path.to_string_lossy());
            }
        }
        1633 | 1656 => browser_setup(state.hwnd, &mut *state.app),
        1863 => {
            let page = state.page(140);
            check(page, 1825, true);
            check(page, 1823, false);
        }
        1777 => info(
            state.hwnd,
            "Enter a host name or an http://, https:// or socks5:// URL in Proxy server address. Set the port and optional website proxy credentials in the adjacent fields. Credentials are retained for this session only. Use system settings to clear the custom proxy.",
        ),
        _ => {}
    }
}
unsafe fn save_options(state: &mut State) -> bool {
    if !save_options_category(state) {
        return false;
    }
    let folder = state.settings.download_dir.to_string_lossy().into_owned();
    if !std::path::Path::new(&folder).is_absolute() {
        info(state.hwnd, "Choose an absolute download directory.");
        return false;
    }
    let mut proxy = String::new();
    let page = state.page(140);
    if checked(page, 1823) {
        let value = get(page, 1103);
        let value = if value.contains("://") {
            value
        } else {
            format!("http://{value}")
        };
        let Ok(mut url) = url::Url::parse(&value) else {
            info(state.hwnd, "Enter a valid HTTP or SOCKS proxy address.");
            return false;
        };
        if !["http", "https", "socks5", "socks5h"].contains(&url.scheme())
            || url.host_str().is_none()
        {
            info(state.hwnd, "Enter a valid HTTP or SOCKS proxy address.");
            return false;
        }
        let port = get(page, 1104);
        if !port.trim().is_empty() {
            let Ok(port) = port.trim().parse::<u16>() else {
                info(
                    state.hwnd,
                    "The proxy port must be a number from 1 to 65535.",
                );
                return false;
            };
            if port == 0 || url.set_port(Some(port)).is_err() {
                info(
                    state.hwnd,
                    "The proxy port must be a number from 1 to 65535.",
                );
                return false;
            }
        }
        let user = get(page, 1105);
        let password = get(page, 1106);
        let _ = url.set_username(&user);
        let _ = url.set_password((!password.is_empty()).then_some(password.as_str()));
        proxy = url.to_string();
    }
    if let Err(error) = std::fs::create_dir_all(&folder) {
        info(
            state.hwnd,
            &format!("Cannot use the download directory: {error}"),
        );
        return false;
    }
    let probe = std::path::Path::new(&folder).join(format!(
        ".idx-write-check-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(file) => {
            drop(file);
            let _ = std::fs::remove_file(probe);
        }
        Err(error) => {
            info(
                state.hwnd,
                &format!("Cannot write to the download directory: {error}"),
            );
            return false;
        }
    }
    let app = &mut *state.app;
    crate::categories::commit(
        &mut app.lib.settings,
        &mut app.lib.downloads,
        &state.settings,
        &mut app.filter,
    );
    app.lib.settings.user_agent = get(state.page(291), 1840);
    app.lib.settings.connections = selected_combo(state.page(141), 1206) + 1;
    app.lib.settings.retries = number(state.page(141), 1074, 3, 0, 10) as u32;
    app.lib.settings.speed_limit_kib = if checked(state.page(141), 1254) {
        number(state.page(141), 1046, 0, 0, 1_024_000)
    } else {
        0
    };
    app.lib.settings.proxy = proxy;
    for (id, control) in &app.controls {
        let limit = app
            .lib
            .downloads
            .iter()
            .find(|d| d.id == *id)
            .and_then(|d| d.speed_limit_kib)
            .unwrap_or(app.lib.settings.speed_limit_kib);
        control.set_speed_limit(limit);
    }
    app.save();
    state.settings = app.lib.settings.clone();
    true
}

unsafe fn init_scheduler(state: &mut State) {
    hide(state.hwnd, &[1676]);
    tabs(
        state,
        &["Schedule", "Files in the queue"],
        (140, 14, 287, 247),
    );
    add_page(state, 306, 143, 29);
    add_page(state, 307, 143, 29);
    enabled(state.hwnd, 1089, true);
    let page = state.page(306);
    disable(
        page,
        &[
            1584, 1595, 1596, 1040, 1590, 1591, 1592, 1227, 1271, 1228, 1235, 1443,
        ],
    );
    hide(page, &[1595, 1596, 1597, 1040, 1598, 1599]);
    check(page, 1679, true);
    set(page, 1680, "Number of retries for failed downloads:");
    check(page, 1680, true);
    enabled(page, 1681, true);
    let files = state.page(307);
    columns(
        GetDlgItem(files, 1025),
        &[("File name", 280), ("Status", 110)],
    );
    SendMessageW(GetDlgItem(files, 1214), 0x46f, 1, 8);
    for (id, label) in [(1229, "↑"), (1230, "↓"), (1231, "×")] {
        let h = GetDlgItem(files, id);
        let style = GetWindowLongPtrW(h, -16);
        SetWindowLongPtrW(h, -16, style & !0x80);
        set(files, id, label);
    }
    refresh_queues(state);
    load_queue(state);
    show_page(state, 0);
}
unsafe fn refresh_queues(state: &mut State) {
    let app = &mut *state.app;
    if app.lib.settings.queues.is_empty() {
        app.lib.settings.queues.push(Queue::default());
    }
    state.queue = state.queue.min(app.lib.settings.queues.len() - 1);
    let tree = GetDlgItem(state.hwnd, 1682);
    SendMessageW(tree, TVM + 1, 0, -65536isize);
    for (i, q) in app.lib.settings.queues.iter().enumerate() {
        let h = tree_item(tree, &q.name, i as isize);
        if i == state.queue {
            SendMessageW(tree, TVM + 11, 9, h as isize);
        }
    }
    enabled(state.hwnd, 1678, app.lib.settings.queues.len() > 1);
}
unsafe fn load_queue(state: &mut State) {
    let app = &*state.app;
    let Some(q) = app.lib.settings.queues.get(state.queue) else {
        return;
    };
    let page = state.page(306);
    set(
        state.hwnd,
        1683,
        &format!("{}{}", q.name, if q.active { " (running)" } else { "" }),
    );
    check(page, 1218, q.scheduled);
    check(page, 1729, q.start_on_startup);
    check(page, 1219, q.schedule_date.is_some());
    check(page, 1221, q.schedule_date.is_none());
    date_set(
        page,
        1220,
        q.schedule_date.unwrap_or_else(|| Local::now().date_naive()),
    );
    check(page, 1225, q.stop_enabled);
    time_set(page, 1216, q.start_hour, q.start_minute);
    time_set(page, 1226, q.stop_hour, q.stop_minute);
    for (id, on) in DAYS.iter().zip(q.weekdays) {
        check(page, *id, on);
    }
    set(page, 1681, &app.lib.settings.retries.to_string());
    check(page, 1680, app.lib.settings.retries > 0);
    enabled(page, 1681, app.lib.settings.retries > 0);
    set(state.page(307), 1036, &q.parallel.to_string());
    schedule_enabled(state);
    refresh_files(state, None);
}
unsafe fn schedule_enabled(state: &State) {
    let page = state.page(306);
    let on = checked(page, 1218);
    for id in DAYS {
        enabled(page, id, on && checked(page, 1221));
    }
    enabled(page, 1216, on);
    enabled(page, 1219, on);
    enabled(page, 1220, on && checked(page, 1219));
    enabled(page, 1221, on);
    enabled(page, 1225, on);
    enabled(page, 1226, on && checked(page, 1225));
}
unsafe fn save_queue(state: &mut State) {
    let page = state.page(306);
    let files = state.page(307);
    let app = &mut *state.app;
    if let Some(q) = app.lib.settings.queues.get_mut(state.queue) {
        let before = (
            q.scheduled,
            q.schedule_date,
            q.start_hour,
            q.start_minute,
            q.stop_enabled,
            q.stop_hour,
            q.stop_minute,
            q.weekdays,
        );
        q.scheduled = checked(page, 1218);
        q.start_on_startup = checked(page, 1729);
        q.schedule_date = if checked(page, 1219) {
            Some(date_get(page, 1220).unwrap_or_else(|| Local::now().date_naive()))
        } else {
            None
        };
        q.stop_enabled = checked(page, 1225);
        (q.start_hour, q.start_minute) = time_get(page, 1216);
        (q.stop_hour, q.stop_minute) = time_get(page, 1226);
        for (i, id) in DAYS.iter().enumerate() {
            q.weekdays[i] = checked(page, *id);
        }
        q.parallel = number(files, 1036, 2, 1, 8) as usize;
        if before
            != (
                q.scheduled,
                q.schedule_date,
                q.start_hour,
                q.start_minute,
                q.stop_enabled,
                q.stop_hour,
                q.stop_minute,
                q.weekdays,
            )
        {
            q.last_schedule_event = None;
        }
    }
    app.lib.settings.retries = if checked(page, 1680) {
        number(page, 1681, 3, 0, 10) as u32
    } else {
        0
    };
}
unsafe fn refresh_files(state: &mut State, selection: Option<usize>) {
    let app = &*state.app;
    let list = GetDlgItem(state.page(307), 1025);
    let Some(q) = app.lib.settings.queues.get(state.queue) else {
        return;
    };
    let snapshot: Vec<_> = app
        .lib
        .downloads
        .iter()
        .filter(|d| d.queue == q.name && d.status != Status::Complete)
        .map(|d| (d.id, d.name(), d.status))
        .collect();
    if snapshot == state.queue_snapshot {
        if let Some(index) = selection.filter(|i| *i < snapshot.len()) {
            select_row(list, index);
        }
        return;
    }
    let selected_id = selected_row(list)
        .and_then(|i| state.queue_rows.get(i))
        .copied();
    SendMessageW(list, LVM + 9, 0, 0);
    state.queue_rows.clear();
    for (id, name, status) in &snapshot {
        let i = state.queue_rows.len();
        row(list, i, name, *id as isize);
        cell(list, i, 1, status.label());
        state.queue_rows.push(*id);
    }
    state.queue_snapshot = snapshot;
    let selection = selection.or_else(|| {
        selected_id.and_then(|id| state.queue_rows.iter().position(|value| *value == id))
    });
    if let Some(index) = selection.filter(|i| *i < state.queue_rows.len()) {
        select_row(list, index);
    }
}
unsafe fn scheduler_command(state: &mut State, id: i32) {
    match id {
        2 | 1079 => close(state),
        1089 => {
            save_queue(state);
            (*state.app).save();
        }
        1232 | 1233 => {
            save_queue(state);
            if id == 1232 {
                (*state.app).start_queue(state.queue);
            } else {
                (*state.app).stop_queue(state.queue);
            }
            (*state.app).save();
            load_queue(state);
        }
        1218 | 1225 => schedule_enabled(state),
        1219 | 1221 => {
            check(state.page(306), 1219, id == 1219);
            check(state.page(306), 1221, id == 1221);
            schedule_enabled(state);
        }
        1680 => {
            let page = state.page(306);
            enabled(page, 1681, checked(page, 1680));
        }
        1675 => {
            save_queue(state);
            if let Some(name) = queue_name(state, "") {
                let app = &mut *state.app;
                if app
                    .lib
                    .settings
                    .queues
                    .iter()
                    .any(|q| q.name.eq_ignore_ascii_case(&name))
                {
                    info(state.hwnd, "A queue with that name already exists.");
                } else {
                    app.lib.settings.queues.push(Queue {
                        name,
                        ..Default::default()
                    });
                    state.queue = app.lib.settings.queues.len() - 1;
                    app.save();
                    refresh_queues(state);
                    load_queue(state);
                }
            }
        }
        1678 => {
            save_queue(state);
            let app = &mut *state.app;
            if app.lib.settings.queues.len() > 1 {
                app.stop_queue(state.queue);
                let old = app.lib.settings.queues.remove(state.queue).name;
                let fallback = app.lib.settings.queues[0].name.clone();
                for download in &mut app.lib.downloads {
                    if download.queue == old {
                        download.queue = fallback.clone();
                    }
                }
                state.queue = 0;
                app.save();
                refresh_queues(state);
                load_queue(state);
            }
        }
        1229 | 1230 | 1231 => {
            let list = GetDlgItem(state.page(307), 1025);
            if let Some(i) = selected_row(list).filter(|i| *i < state.queue_rows.len()) {
                let app = &mut *state.app;
                let Some(from) = app
                    .lib
                    .downloads
                    .iter()
                    .position(|d| d.id == state.queue_rows[i])
                else {
                    return;
                };
                if id == 1231 {
                    let download = &mut app.lib.downloads[from];
                    download.queue = String::new();
                    if download.status == Status::Queued {
                        download.status = Status::Paused;
                    }
                    app.save();
                    refresh_files(state, None);
                } else {
                    let other = if id == 1229 {
                        i.checked_sub(1)
                    } else {
                        (i + 1 < state.queue_rows.len()).then_some(i + 1)
                    };
                    if let Some(j) = other {
                        if let Some(to) = app
                            .lib
                            .downloads
                            .iter()
                            .position(|d| d.id == state.queue_rows[j])
                        {
                            app.lib.downloads.swap(from, to);
                            app.save();
                            refresh_files(state, Some(j));
                        }
                    }
                }
            }
        }
        1171 => info(
            state.hwnd,
            "Choose Once at for a single date, or Daily and the required weekdays. A stop time earlier than the start time stops on the following day. IDX must be running; delayed ticks catch up after sleep. Start download on IDX startup resumes this queue when IDX opens. Double-click a queue name or press F2 to rename it. Files in the queue controls parallel downloads and order. Click Apply to save.",
        ),
        _ => {}
    }
}
struct QueueName {
    app: *mut IdxApp,
    context: egui::Context,
    result: Option<String>,
    busy: bool,
    initial: String,
}
unsafe fn rename_queue(state: &mut State) {
    save_queue(state);
    let Some(queue) = (&(*state.app).lib.settings.queues).get(state.queue) else {
        return;
    };
    let initial = queue.name.clone();
    if let Some(name) = queue_name(state, &initial) {
        match (*state.app).rename_queue(state.queue, &name) {
            Ok(()) => {
                refresh_queues(state);
                load_queue(state);
            }
            Err(error) => info(state.hwnd, &error),
        }
    }
}
unsafe fn queue_name(state: &mut State, initial: &str) -> Option<String> {
    let mut q = QueueName {
        app: state.app,
        context: state.context.clone(),
        result: None,
        busy: false,
        initial: initial.to_string(),
    };
    let template = super::native_resources::top_level(310);
    DialogBoxIndirectParamW(
        GetModuleHandleW(null()),
        template.as_ptr().cast(),
        state.hwnd,
        Some(queue_proc),
        (&mut q as *mut QueueName) as isize,
    );
    q.result
}
unsafe extern "system" fn queue_proc(hwnd: Handle, msg: u32, w: usize, l: isize) -> isize {
    if msg == WM_INIT {
        SetWindowLongPtrW(hwnd, USERDATA, l);
        let q = &*(l as *const QueueName);
        SendMessageW(GetDlgItem(hwnd, 1660), 0xc5, 128, 0);
        if !q.initial.is_empty() {
            set(hwnd, 0, "Rename queue");
            set(hwnd, 1686, "Enter the new name for this queue");
            set(hwnd, 1660, &q.initial);
        }
        icon(hwnd);
        center(hwnd, GetParent(hwnd));
        SetTimer(hwnd, 1, 150, null());
        return 1;
    }
    let ptr = GetWindowLongPtrW(hwnd, USERDATA) as *mut QueueName;
    if ptr.is_null() || (*ptr).busy {
        return 0;
    }
    (*ptr).busy = true;
    let q = &mut *ptr;
    let result = match msg {
        WM_COMMAND => {
            match w & 0xffff {
                1 => {
                    let value = get(hwnd, 1660).trim().to_string();
                    if value.is_empty()
                        || value.chars().count() > 128
                        || value.chars().any(char::is_control)
                    {
                        info(
                            hwnd,
                            "Enter a queue name of 1 to 128 characters without control characters.",
                        );
                    } else {
                        q.result = Some(value);
                        EndDialog(hwnd, 1);
                    }
                }
                2 => {
                    EndDialog(hwnd, 0);
                }
                _ => {}
            }
            1
        }
        WM_TIMER => {
            (*q.app).tick(&q.context);
            (*q.app).sync_native_progress();
            1
        }
        WM_CLOSE => {
            EndDialog(hwnd, 0);
            1
        }
        2 => {
            KillTimer(hwnd, 1);
            0
        }
        _ => 0,
    };
    (*ptr).busy = false;
    result
}

unsafe fn init_grabber(state: &mut State) {
    set(state.hwnd, 0, "IDX Site Grabber");
    state.illustration = Illustration::load();
    for id in [257, 259, 264, 240] {
        let page = add_page(state, id, 0, 0);
        if let Some(picture) = &state.illustration {
            let panel = GetDlgItem(page, 1801);
            SetWindowLongPtrW(panel, USERDATA, picture.image as isize);
            let style = GetWindowLongPtrW(panel, -16);
            SetWindowLongPtrW(panel, -16, (style & !0x1f) | 0x0d);
        }
    }
    let intro = state.page(257);
    set(intro, 1411, "Site links");
    set(intro, 1402, &(*state.app).grab_url);
    combo(intro, 1443, &["Find downloadable links on HTML pages"], 0);
    disable(intro, &[1411, 1443, 1439, 1033, 1041, 1485]);
    hide(intro, &[1441, 1070, 1561, 1562, 1564]);
    set(
        intro,
        1470,
        "Find links and media in HTML pages. Choose how many linked pages to explore in the Exploration step.",
    );
    set(
        intro,
        1563,
        "Website login is not available in this page scanner.",
    );
    let save = state.page(259);
    check(save, 1359, true);
    combo(
        save,
        1204,
        &[
            "Compressed",
            "Documents",
            "Music",
            "Programs",
            "Video",
            "Other",
        ],
        0,
    );
    set(
        save,
        1031,
        &(*state.app).lib.settings.download_dir.to_string_lossy(),
    );
    disable(save, &[1357, 1358, 1204, 1492, 1514, 1468, 1469]);
    set(
        save,
        1359,
        "Save to this base folder (current category rules apply)",
    );
    check(save, 1436, true);
    enabled(save, 1436, false);
    let explore = state.page(264);
    disable(explore, &[1494, 1493, 1565, 1040, 1070]);
    check(explore, 1445, true);
    set(explore, 1406, "1");
    set(explore, 1472, "0");
    set(
        explore,
        1512,
        "Choose which pages to explore. Each scan is limited to 250 requests and 20,000 links. JavaScript is not executed.",
    );
    set(
        explore,
        1489,
        "Level 0 reads the start page; level 1 follows its page links.",
    );
    let filter = state.page(240);
    filter_combos(state, [0, 0]);
    combo(filter, 1443, &["Bytes", "KB", "MB", "GB"], 1);
    combo(filter, 1444, &["Bytes", "KB", "MB", "GB"], 2);
    set(filter, 1479, "0");
    set(filter, 1481, "0");
    disable(filter, &[1459, 1479, 1443, 1481, 1444, 1040, 1041]);
    check(filter, 1571, true);
    set(
        filter,
        1471,
        "After scanning, check the files to add to the IDX download list. Choose Download later or Start Download in the download information dialog.",
    );
    state.wizard_step = 0;
    show_page(state, 0);
    center(state.hwnd, state.owner);
    enabled(state.hwnd, 1429, false);
}
unsafe fn grabber_command(state: &mut State, id: i32) {
    match id {
        2 | 1079 => close(state),
        1485 if !state.scanning && state.active >= 2 => {
            state.advanced[state.active] = !state.advanced[state.active];
            layout_grabber(state);
        }
        1474 | 1476 => {
            let page = state.page(264);
            enabled(
                page,
                if id == 1474 { 1040 } else { 1070 },
                checked(page, id),
            );
        }
        1480 | 1482 | 1483 | 1486 => {
            let page = state.page(240);
            let targets: &[i32] = match id {
                1480 => &[1479, 1443],
                1482 => &[1481, 1444],
                1483 => &[1040],
                _ => &[1041],
            };
            for target in targets {
                enabled(page, *target, checked(page, id));
            }
        }
        1405 | 1421 if !state.scanning => {
            let filter = state.page(240);
            let old = [selected_combo(filter, 1404), selected_combo(filter, 1420)];
            match grabber_filters::edit(state.hwnd, state.app, &state.settings.grabber_filters) {
                Ok(Some(edited)) => {
                    let selected = old.map(|i| {
                        i.checked_sub(1)
                            .and_then(|old| {
                                edited
                                    .origins
                                    .iter()
                                    .position(|origin| *origin == Some(old))
                            })
                            .map_or(0, |i| i + 1)
                    });
                    state.settings.grabber_filters = edited.filters;
                    (*state.app).lib.settings.grabber_filters =
                        state.settings.grabber_filters.clone();
                    (*state.app).save();
                    filter_combos(state, selected);
                }
                Ok(None) => {}
                Err(error) => info(state.hwnd, &error),
            }
        }
        1435 | 1445 => {
            let page = state.page(264);
            check(page, 1435, id == 1435);
            check(page, 1445, id == 1445);
            for control in [1406, 1472, 1490, 1491] {
                enabled(page, control, id == 1445);
            }
        }
        1360 => {
            let page = state.page(259);
            if let Some(path) = rfd::FileDialog::new()
                .set_directory(get(page, 1031))
                .pick_folder()
            {
                set(page, 1031, &path.to_string_lossy());
            }
        }
        1429 => {
            if state.wizard_step > 0 && !state.scanning {
                state.wizard_step -= 1;
                show_page(state, state.wizard_step);
                enabled(state.hwnd, 1429, state.wizard_step > 0);
                set(state.hwnd, 1430, "Next >");
            }
        }
        1430 => {
            if state.scanning {
                return;
            }
            let intro = state.page(257);
            let url = get(intro, 1402).trim().to_string();
            if crate::model::filename_from_url(&url).is_err() {
                info(state.hwnd, "Enter an HTTP or HTTPS starting page address.");
                return;
            }
            (*state.app).grab_url = url;
            if state.wizard_step < 3 {
                state.wizard_step += 1;
                show_page(state, state.wizard_step);
                enabled(state.hwnd, 1429, true);
                set(
                    state.hwnd,
                    1430,
                    if state.wizard_step == 3 {
                        "Scan site"
                    } else {
                        "Next >"
                    },
                );
            } else {
                let explore = state.page(264);
                let whole_site = checked(explore, 1435);
                let parse_level = |id| {
                    get(explore, id)
                        .trim()
                        .parse::<usize>()
                        .ok()
                        .filter(|n| *n <= 100)
                };
                let (levels, other_levels) = if whole_site {
                    (usize::MAX, 0)
                } else if let (Some(levels), Some(other)) = (parse_level(1406), parse_level(1472)) {
                    (levels, other)
                } else {
                    info(
                        state.hwnd,
                        "Enter link levels from 0 to 100 in the Exploration step.",
                    );
                    return;
                };
                let filter = state.page(240);
                let page_paths = PathRules {
                    include: checked_text(explore, 1474, 1040),
                    exclude: checked_text(explore, 1476, 1070),
                };
                let size = |toggle, value, unit| -> Result<Option<u64>, String> {
                    if checked(filter, toggle) {
                        size_bytes(&get(filter, value), selected_combo(filter, unit)).map(Some)
                    } else {
                        Ok(None)
                    }
                };
                let sizes = size(1480, 1479, 1443)
                    .and_then(|min| size(1482, 1481, 1444).map(|max| (min, max)));
                let (min_size, max_size) = match sizes {
                    Ok(sizes) => sizes,
                    Err(error) => {
                        info(state.hwnd, &error);
                        return;
                    }
                };
                let files = FileRules {
                    include: filter_mask(state, 1404),
                    exclude: filter_mask(state, 1420),
                    site_only: checked(filter, 1437),
                    paths: PathRules {
                        include: checked_text(filter, 1483, 1040),
                        exclude: checked_text(filter, 1486, 1041),
                    },
                    min_size,
                    max_size,
                    hide_duplicates: checked(filter, 1571),
                };
                if let Err(error) = Paths::new(&page_paths).and_then(|_| Files::new(&files)) {
                    info(state.hwnd, &error);
                    return;
                }
                let save = state.page(259);
                state.category = 0;
                state.folder = get(save, 1031);
                if !std::path::Path::new(&state.folder).is_absolute() {
                    info(
                        state.hwnd,
                        "Choose an absolute destination folder in the Save To step.",
                    );
                    return;
                }
                (*state.app).start_grabber(crate::grabber::ScanOptions {
                    levels,
                    other_levels,
                    no_parent_directories: checked(explore, 1416),
                    page_paths,
                    files,
                    ..Default::default()
                });
                state.scanning = true;
                enabled(filter, 1405, false);
                enabled(filter, 1421, false);
                enabled(state.hwnd, 1430, false);
                enabled(state.hwnd, 1429, false);
                set(state.hwnd, 1430, "Scanning...");
            }
        }
        _ => {}
    }
}
unsafe fn checked_text(page: Handle, toggle: i32, edit: i32) -> String {
    if checked(page, toggle) {
        get(page, edit)
    } else {
        String::new()
    }
}
unsafe fn filter_mask(state: &State, id: i32) -> String {
    selected_combo(state.page(240), id)
        .checked_sub(1)
        .and_then(|i| state.settings.grabber_filters.get(i))
        .map(|f| f.mask.clone())
        .unwrap_or_default()
}
unsafe fn filter_combos(state: &State, selected: [usize; 2]) {
    for (i, (id, first)) in [(1404, "All files"), (1420, "None")].iter().enumerate() {
        let labels = std::iter::once(first.to_string())
            .chain(
                state
                    .settings
                    .grabber_filters
                    .iter()
                    .map(|f| format!("{}  ( {} )", f.name, f.mask)),
            )
            .collect::<Vec<_>>();
        combo(
            state.page(240),
            *id,
            &labels.iter().map(String::as_str).collect::<Vec<_>>(),
            selected[i],
        );
    }
}
/// Keep original control positions; expanding reveals the original 274-DU pages
/// and restores the original 303-DU host height (FUN_0053e250).
unsafe fn layout_grabber(state: &State) {
    let expanded = state.advanced[state.active];
    for (index, (id, page)) in state.pages.iter().enumerate() {
        let advanced = state.advanced[index];
        let controls: &[i32] = match id {
            264 => &[1474, 1040, 1476, 1070, 1488],
            240 => &[
                1484, 1517, 1480, 1479, 1443, 1482, 1481, 1444, 1483, 1040, 1486, 1041, 1488,
            ],
            _ => &[],
        };
        for id in controls {
            ShowWindow(GetDlgItem(*page, *id), if advanced { 5 } else { 0 });
        }
        if index >= 2 {
            set(
                *page,
                1485,
                if advanced {
                    "Advanced <<"
                } else {
                    "Advanced >>"
                },
            );
        }
        let size = rect(state.hwnd, 0, 0, 406, if advanced { 274 } else { 179 });
        SetWindowPos(*page, null_mut(), 0, 0, size.right, size.bottom, 0x16);
    }
    let extra = if expanded { 92 } else { 0 };
    for (id, x, y, w, h) in [
        (1487, 7, 183, 392, 1),
        (1429, 155, 190, 60, 14),
        (1430, 221, 190, 60, 14),
        (1079, 324, 190, 60, 14),
    ] {
        let r = rect(state.hwnd, x, y + extra, w, h);
        SetWindowPos(
            GetDlgItem(state.hwnd, id),
            null_mut(),
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            0x14,
        );
    }
    let (mut outer, mut client) = (Rect::default(), Rect::default());
    GetWindowRect(state.hwnd, &mut outer);
    GetClientRect(state.hwnd, &mut client);
    let height = rect(state.hwnd, 0, 0, 406, 211 + extra).bottom;
    SetWindowPos(
        state.hwnd,
        null_mut(),
        0,
        0,
        outer.right - outer.left,
        height + outer.bottom - outer.top - client.bottom,
        0x16,
    );
}
unsafe fn init_results(state: &mut State) {
    set(state.hwnd, 0, "IDX Site Grabber — results");
    set(state.hwnd, 1429, "Check all");
    set(state.hwnd, 1430, "Add selected");
    for id in [1501, 1502, 1503, 1505, 1506] {
        hide(state.hwnd, &[id]);
    }
    let tree = GetDlgItem(state.hwnd, 1431);
    tree_item(tree, "Page links", 0);
    let list = GetDlgItem(state.hwnd, 1023);
    columns(list, &[("URL", 510), ("Type", 65)]);
    SendMessageW(list, LVM + 54, 0, 0x10024);
    let app = &mut *state.app;
    state.result_rows.clear();
    for (i, (_, url)) in app.grab_links.iter().enumerate() {
        {
            let n = state.result_rows.len();
            row(list, n, url, i as isize);
            let ext = url::Url::parse(url)
                .ok()
                .and_then(|u| u.path().rsplit('.').next().map(str::to_string))
                .unwrap_or_default();
            cell(list, n, 1, &ext);
            state.result_rows.push(i);
        }
    }
    control(
        state.hwnd,
        "Static",
        &format!("{} matched; {}", state.result_rows.len(), app.grab_status),
        19510,
        0,
        (8, 265, 235, 14),
    );
}
unsafe fn results_command(state: &mut State, id: i32) {
    let list = GetDlgItem(state.hwnd, 1023);
    match id {
        2 | 1079 => close(state),
        1429 => {
            let any_unchecked = (0..state.result_rows.len())
                .any(|i| SendMessageW(list, LVM + 44, i, 0xf000) & 0xf000 != 0x2000);
            for i in 0..state.result_rows.len() {
                let item = ListItem {
                    state: if any_unchecked { 0x2000 } else { 0x1000 },
                    state_mask: 0xf000,
                    ..Default::default()
                };
                SendMessageW(list, LVM + 43, i, (&item as *const ListItem) as isize);
            }
            set(
                state.hwnd,
                1429,
                if any_unchecked {
                    "Uncheck all"
                } else {
                    "Check all"
                },
            );
        }
        1430 => {
            let app = &mut *state.app;
            let urls = state
                .result_rows
                .iter()
                .enumerate()
                .filter(|(i, _)| SendMessageW(list, LVM + 44, *i, 0xf000) & 0xf000 == 0x2000)
                .map(|(_, i)| app.grab_links[*i].1.clone())
                .collect::<Vec<_>>();
            if urls.is_empty() {
                info(state.hwnd, "Check at least one link to add.");
                return;
            }
            app.add_dialog(urls.join("\n"));
            app.add.parent_page = app.grab_page.clone();
            app.add.parent_pages = app.grab_parents.clone();
            if !state.folder.is_empty() {
                app.add.folder = state.folder.clone();
            }
            app.add.category = state.category;
            app.add.start = false;
            EndDialog(state.hwnd, 1);
        }
        _ => {}
    }
}
