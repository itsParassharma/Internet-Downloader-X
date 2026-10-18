//! Native, modeless download window instantiated from the extracted RT_DIALOG resources.
//! The resource controls keep their original IDs, Tahoma font and dialog-unit positions.
//! All HWNDs and GDI objects remain on the dialog thread; the downloader exchanges data only.

use crate::model::{Download, Status};
use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

#[derive(Debug)]
pub enum Action {
    Pause(u64),
    Resume(u64),
    Cancel(u64),
    Closed,
    Error(String),
    Preferences {
        id: u64,
        speed_limit_kib: Option<u64>,
        notify_on_complete: bool,
        open_folder_on_complete: bool,
        close_progress_on_complete: bool,
    },
}

#[derive(Clone)]
struct Snapshot {
    download: Download,
    global_limit: u64,
}
#[derive(Default)]
struct Shared {
    latest: Option<Snapshot>,
    visible: bool,
    generation: u64,
    exit: bool,
    actions: Vec<Action>,
}

pub struct NativeProgress {
    shared: Arc<Mutex<Shared>>,
    worker: Option<JoinHandle<()>>,
}
impl NativeProgress {
    pub fn new() -> Self {
        let shared = Arc::new(Mutex::new(Shared::default()));
        let thread_shared = shared.clone();
        let worker = thread::Builder::new()
            .name("idx-native-progress".into())
            .spawn(move || run_dialog(thread_shared));
        let worker = match worker {
            Ok(worker) => Some(worker),
            Err(error) => {
                lock(&shared).actions.push(Action::Error(format!(
                    "Could not open the download progress window: {error}"
                )));
                None
            }
        };
        Self { shared, worker }
    }

    pub fn show(&mut self, download: &Download, global_limit: u64) {
        let mut shared = lock(&self.shared);
        if !shared.visible || shared.latest.as_ref().map(|s| s.download.id) != Some(download.id) {
            shared.generation = shared.generation.wrapping_add(1);
        }
        shared.latest = Some(Snapshot {
            download: download.clone(),
            global_limit,
        });
        shared.visible = true;
    }

    pub fn hide(&mut self) {
        lock(&self.shared).visible = false;
    }

    pub fn poll(&mut self) -> Vec<Action> {
        std::mem::take(&mut lock(&self.shared).actions)
    }
}
impl Drop for NativeProgress {
    fn drop(&mut self) {
        lock(&self.shared).exit = true;
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

type Handle = *mut c_void;
type DialogProc = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;
type SubclassProc = unsafe extern "system" fn(Handle, u32, usize, isize, usize, usize) -> isize;
const WM_INITDIALOG: u32 = 0x0110;
const WM_COMMAND: u32 = 0x0111;
const WM_TIMER: u32 = 0x0113;
const WM_NOTIFY: u32 = 0x004e;
const WM_CLOSE: u32 = 0x0010;
const WM_DESTROY: u32 = 0x0002;
const WM_PAINT: u32 = 0x000f;
const WM_NCDESTROY: u32 = 0x0082;
const WM_CTLCOLORDLG: u32 = 0x0136;
const WM_CTLCOLORSTATIC: u32 = 0x0138;
const WM_CTLCOLORBTN: u32 = 0x0135;
const WM_SETFONT: u32 = 0x0030;
const WM_GETFONT: u32 = 0x0031;
const WM_SETICON: u32 = 0x0080;
const WM_USER: u32 = 0x0400;
const LVM: u32 = 0x1000;
const TCM: u32 = 0x1300;
const TAB_ID: i32 = 19000;
const GWLP_USERDATA: i32 = -21;
const GWL_STYLE: i32 = -16;
const SW_HIDE: i32 = 0;
const SW_SHOW: i32 = 5;
const WS_CHILD: u32 = 0x4000_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const WS_TABSTOP: u32 = 0x0001_0000;
const WS_CLIPSIBLINGS: u32 = 0x0400_0000;
const WS_CLIPCHILDREN: u32 = 0x0200_0000;
const SWP_NOACTIVATE: u32 = 0x0010;
const SWP_NOZORDER: u32 = 0x0004;
const SWP_NOMOVE: u32 = 0x0002;

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
#[repr(C)]
#[derive(Default)]
struct Point {
    x: i32,
    y: i32,
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
struct Paint {
    dc: Handle,
    erase: i32,
    rect: Rect,
    restore: i32,
    update: i32,
    reserved: [u8; 32],
}

#[link(name = "user32")]
unsafe extern "system" {
    fn CreateDialogIndirectParamW(
        instance: Handle,
        template: *const c_void,
        parent: Handle,
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
    fn IsDialogMessageW(dialog: Handle, message: *mut Message) -> i32;
    fn PostQuitMessage(code: i32);
    fn SetWindowLongPtrW(window: Handle, index: i32, value: isize) -> isize;
    fn GetWindowLongPtrW(window: Handle, index: i32) -> isize;
    fn GetDlgItem(window: Handle, id: i32) -> Handle;
    fn GetDlgCtrlID(window: Handle) -> i32;
    fn SetWindowTextW(window: Handle, text: *const u16) -> i32;
    fn GetWindowTextW(window: Handle, text: *mut u16, length: i32) -> i32;
    fn GetWindowTextLengthW(window: Handle) -> i32;
    fn SendMessageW(window: Handle, message: u32, w_param: usize, l_param: isize) -> isize;
    fn ShowWindow(window: Handle, command: i32) -> i32;
    fn EnableWindow(window: Handle, enabled: i32) -> i32;
    fn GetFocus() -> Handle;
    fn GetSysColorBrush(index: i32) -> Handle;
    fn SetForegroundWindow(window: Handle) -> i32;
    fn SetWindowPos(
        window: Handle,
        after: Handle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    fn GetWindowRect(window: Handle, rect: *mut Rect) -> i32;
    fn GetClientRect(window: Handle, rect: *mut Rect) -> i32;
    fn MapDialogRect(window: Handle, rect: *mut Rect) -> i32;
    fn MapWindowPoints(from: Handle, to: Handle, points: *mut Point, count: u32) -> i32;
    fn SetTimer(window: Handle, id: usize, milliseconds: u32, timer_proc: *const c_void) -> usize;
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
    fn BeginPaint(window: Handle, paint: *mut Paint) -> Handle;
    fn EndPaint(window: Handle, paint: *const Paint) -> i32;
    fn FillRect(dc: Handle, rect: *const Rect, brush: Handle) -> i32;
    fn FrameRect(dc: Handle, rect: *const Rect, brush: Handle) -> i32;
    fn EnumWindows(
        proc: Option<unsafe extern "system" fn(Handle, isize) -> i32>,
        param: isize,
    ) -> i32;
    fn GetWindowThreadProcessId(window: Handle, process: *mut u32) -> u32;
    fn IsWindowVisible(window: Handle) -> i32;
    fn GetWindow(window: Handle, command: u32) -> Handle;
    fn SetThreadDpiAwarenessContext(context: Handle) -> Handle;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> Handle;
    fn GetCurrentProcessId() -> u32;
    fn GetLastError() -> u32;
}
#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateSolidBrush(color: u32) -> Handle;
    fn DeleteObject(object: Handle) -> i32;
    fn GetStockObject(object: i32) -> Handle;
    fn SetTextColor(dc: Handle, color: u32) -> u32;
    fn SetBkColor(dc: Handle, color: u32) -> u32;
    fn SetBkMode(dc: Handle, mode: i32) -> i32;
}
#[link(name = "comctl32")]
unsafe extern "system" {
    fn InitCommonControlsEx(controls: *const InitControls) -> i32;
    fn SetWindowSubclass(
        window: Handle,
        proc: Option<SubclassProc>,
        id: usize,
        reference: usize,
    ) -> i32;
    fn RemoveWindowSubclass(window: Handle, proc: Option<SubclassProc>, id: usize) -> i32;
    fn DefSubclassProc(window: Handle, message: u32, w_param: usize, l_param: isize) -> isize;
}
#[link(name = "uxtheme")]
unsafe extern "system" {
    fn SetWindowTheme(window: Handle, application: *const u16, id_list: *const u16) -> i32;
}

struct DialogState {
    shared: Arc<Mutex<Shared>>,
    window: Cell<Handle>,
    pages: Cell<[Handle; 3]>,
    tab: Cell<Handle>,
    snapshot: RefCell<Option<Snapshot>>,
    visible: Cell<bool>,
    generation: Cell<u64>,
    details: Cell<bool>,
    full_height: Cell<i32>,
    syncing: Cell<bool>,
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn text(window: Handle, value: &str) {
    let encoded = wide(value);
    // Reassigning unchanged static captions causes avoidable erases on classic controls.
    if unsafe { GetWindowTextLengthW(window) } == encoded.len().saturating_sub(1) as i32 {
        let mut current = vec![0u16; encoded.len()];
        unsafe {
            GetWindowTextW(window, current.as_mut_ptr(), current.len() as i32);
        }
        if current == encoded {
            return;
        }
    }
    unsafe {
        SetWindowTextW(window, encoded.as_ptr());
    }
}
fn child(window: Handle, id: i32) -> Handle {
    unsafe { GetDlgItem(window, id) }
}
fn control_text(window: Handle, id: i32, value: &str) {
    text(child(window, id), value);
}
fn send(window: Handle, message: u32, w_param: usize, l_param: isize) -> isize {
    unsafe { SendMessageW(window, message, w_param, l_param) }
}
fn check(window: Handle, id: i32, enabled: bool) {
    send(child(window, id), 0x00f1, usize::from(enabled), 0);
}
fn checked(window: Handle, id: i32) -> bool {
    send(child(window, id), 0x00f0, 0, 0) == 1
}
fn show(window: Handle, visible: bool) {
    unsafe {
        ShowWindow(window, if visible { SW_SHOW } else { SW_HIDE });
    }
}
fn enable(window: Handle, enabled: bool) {
    unsafe {
        EnableWindow(window, i32::from(enabled));
    }
}
fn du(window: Handle, left: i32, top: i32, right: i32, bottom: i32) -> Rect {
    let mut rect = Rect {
        left,
        top,
        right,
        bottom,
    };
    unsafe {
        MapDialogRect(window, &mut rect);
    }
    rect
}

/// The backing allocation is DWORD-aligned, as required by CreateDialogIndirectParamW.
/// Only the parent/window styles change; every extracted control and font stays intact.
fn template(resource: &[u8], page: bool) -> Vec<u32> {
    let mut bytes = resource.to_vec();
    let original = u32::from_le_bytes(bytes[12..16].try_into().expect("DIALOGEX style"));
    let style = if page {
        // DS_CONTROL; remove popup, caption, system menu, thick frame and initial visibility.
        (original | WS_CHILD | 0x0400 | WS_CLIPSIBLINGS)
            & !(0x8000_0000 | 0x00c0_0000 | 0x000c_0000 | WS_VISIBLE)
    } else {
        (original | 0x00c8_0000 | 0x0002_0000 | WS_CLIPCHILDREN)
            & !(0x0004_0000 | 0x0001_0000 | WS_VISIBLE)
    };
    bytes[12..16].copy_from_slice(&style.to_le_bytes());
    if !page {
        // PBS_SMOOTH is read when the original progress control is created.
        let progress_style = u32::from_le_bytes(
            bytes[0x60..0x64]
                .try_into()
                .expect("Progress control style"),
        ) | 1;
        bytes[0x60..0x64].copy_from_slice(&progress_style.to_le_bytes());
    }
    if page {
        let exstyle =
            u32::from_le_bytes(bytes[8..12].try_into().expect("DIALOGEX exstyle")) | 0x0001_0000;
        bytes[8..12].copy_from_slice(&exstyle.to_le_bytes());
    }
    bytes.resize(bytes.len().div_ceil(4) * 4, 0);
    bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().expect("DWORD")))
        .collect()
}

unsafe extern "system" fn find_owner(window: Handle, result: isize) -> i32 {
    let mut process = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut process);
    }
    if process != unsafe { GetCurrentProcessId() }
        || unsafe { IsWindowVisible(window) } == 0
        || !unsafe { GetWindow(window, 4) }.is_null()
    {
        return 1;
    }
    let mut title = [0u16; 256];
    let count = unsafe { GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32) };
    let title = String::from_utf16_lossy(&title[..count.max(0) as usize]);
    if title.contains("IDX") && title.contains("Internet Downloader X") {
        unsafe {
            *(result as *mut Handle) = window;
        }
        return 0;
    }
    1
}

fn run_dialog(shared: Arc<Mutex<Shared>>) {
    // Creating and pumping the entire modeless window on this thread avoids blocking eframe.
    unsafe {
        SetThreadDpiAwarenessContext((-4isize) as Handle);
        InitCommonControlsEx(&InitControls {
            size: std::mem::size_of::<InitControls>() as u32,
            classes: 0x0000_ffff,
        });
    }
    let state = Box::new(DialogState {
        shared,
        window: Cell::new(null_mut()),
        pages: Cell::new([null_mut(); 3]),
        tab: Cell::new(null_mut()),
        snapshot: RefCell::new(None),
        visible: Cell::new(false),
        generation: Cell::new(0),
        details: Cell::new(true),
        full_height: Cell::new(0),
        syncing: Cell::new(false),
    });
    let pointer = (&*state as *const DialogState) as isize;
    let instance = unsafe { GetModuleHandleW(null()) };
    let mut owner: Handle = null_mut();
    unsafe {
        EnumWindows(Some(find_owner), (&mut owner as *mut Handle) as isize);
    }
    let resource = template(include_bytes!("../../assets/idm/dialog_103.bin"), false);
    let window = unsafe {
        CreateDialogIndirectParamW(
            instance,
            resource.as_ptr().cast(),
            owner,
            Some(dialog_proc),
            pointer,
        )
    };
    if window.is_null() {
        lock(&state.shared).actions.push(Action::Error(format!(
            "Could not create the native download window (Windows error {}).",
            unsafe { GetLastError() }
        )));
        return;
    }
    state.window.set(window);
    if let Err(error) = initialize(&state, instance, pointer) {
        lock(&state.shared).actions.push(Action::Error(error));
        unsafe {
            DestroyWindow(window);
        }
        return;
    }
    if unsafe { SetTimer(window, 1, 100, null()) } == 0 {
        lock(&state.shared).actions.push(Action::Error(
            "Could not start the download window refresh timer.".into(),
        ));
        unsafe {
            DestroyWindow(window);
        }
        return;
    }
    let mut message: Message = unsafe { std::mem::zeroed() };
    while unsafe { GetMessageW(&mut message, null_mut(), 0, 0) } > 0 {
        if unsafe { IsDialogMessageW(window, &mut message) } == 0 {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}

fn initialize(state: &DialogState, instance: Handle, pointer: isize) -> Result<(), String> {
    let window = state.window.get();
    text(window, "Download progress — IDX");
    for size in [0usize, 1] {
        let icon = unsafe {
            LoadImageW(
                instance,
                101usize as *const u16,
                1,
                if size == 0 { 16 } else { 32 },
                if size == 0 { 16 } else { 32 },
                0x8000,
            )
        };
        if !icon.is_null() {
            send(window, WM_SETICON, size, icon as isize);
        }
    }
    show(child(window, 1760), false);
    show(child(window, 1853), false);
    let rect = du(window, 7, 5, 345, 106);
    let tab = unsafe {
        CreateWindowExW(
            0,
            wide("SysTabControl32").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_CLIPSIBLINGS,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            window,
            TAB_ID as usize as Handle,
            instance,
            null_mut(),
        )
    };
    if tab.is_null() {
        return Err("Could not create the download status tabs.".into());
    }
    state.tab.set(tab);
    send(tab, WM_SETFONT, send(window, WM_GETFONT, 0, 0) as usize, 1);
    for (index, title) in ["Download status", "Speed Limiter", "Options on completion"]
        .iter()
        .enumerate()
    {
        let mut title = wide(title);
        let mut item: TabItem = unsafe { std::mem::zeroed() };
        item.mask = 1;
        item.text = title.as_mut_ptr();
        send(tab, TCM + 62, index, (&mut item as *mut TabItem) as isize);
    }
    let mut body = Rect::default();
    unsafe {
        GetClientRect(tab, &mut body);
    }
    send(tab, TCM + 40, 0, (&mut body as *mut Rect) as isize);
    unsafe {
        MapWindowPoints(tab, window, (&mut body as *mut Rect).cast(), 2);
    }
    let resources: [&[u8]; 3] = [
        include_bytes!("../../assets/idm/dialog_363.bin"),
        include_bytes!("../../assets/idm/dialog_365.bin"),
        include_bytes!("../../assets/idm/dialog_364.bin"),
    ];
    let mut pages = [null_mut(); 3];
    for (index, bytes) in resources.into_iter().enumerate() {
        let page_template = template(bytes, true);
        let page = unsafe {
            CreateDialogIndirectParamW(
                instance,
                page_template.as_ptr().cast(),
                window,
                Some(page_proc),
                pointer,
            )
        };
        if page.is_null() {
            return Err(format!(
                "Could not create download tab {} (Windows error {}).",
                index + 1,
                unsafe { GetLastError() }
            ));
        }
        unsafe {
            SetWindowPos(
                page,
                null_mut(),
                body.left,
                body.top,
                body.right - body.left,
                body.bottom - body.top,
                SWP_NOACTIVATE,
            );
        }
        pages[index] = page;
        show(page, index == 0);
    }
    state.pages.set(pages);
    // The original completion page contains modem/power operations. Reuse the first two
    // option rows for the implemented IDX actions, and hide unsupported power controls.
    control_text(pages[2], 1761, "After download");
    control_text(pages[2], 1846, "Open the containing folder when done");
    control_text(pages[2], 1847, "Close this progress window when done");
    for id in [1228, 1443, 1235, 1848, 1845] {
        show(child(pages[2], id), false);
    }
    for page in [pages[1], pages[2]] {
        control_text(page, 1772, "Status tab");
    }
    let global_button = child(pages[1], 1765);
    let style = unsafe { GetWindowLongPtrW(global_button, GWL_STYLE) } as u32;
    unsafe {
        SetWindowLongPtrW(global_button, GWL_STYLE, (style & !0x0000_200f) as isize);
    }
    text(global_button, "Use global setting");
    // Keep the original bar geometry, while rendering its continuous green style.
    let progress = child(window, 1146);
    let style = unsafe { GetWindowLongPtrW(progress, GWL_STYLE) } as u32 | 1;
    unsafe {
        SetWindowLongPtrW(progress, GWL_STYLE, style as isize);
        SetWindowTheme(progress, wide("").as_ptr(), wide("").as_ptr());
    }
    send(progress, WM_USER + 6, 0, 10000);
    send(progress, WM_USER + 9, 0, color(0, 180, 0) as isize);
    send(progress, 0x2001, 0, color(230, 230, 230) as isize);
    let list = child(window, 1177);
    send(list, LVM + 1, 0, color(190, 184, 194) as isize);
    send(list, LVM + 38, 0, color(190, 184, 194) as isize);
    send(list, LVM + 36, 0, color(0, 0, 100) as isize);
    send(list, LVM + 54, 0, 0x0000_0020 | 0x0001_0000); // full-row selection, double buffer
    let mut list_rect = Rect::default();
    unsafe {
        GetClientRect(list, &mut list_rect);
    }
    let number_width = du(window, 0, 0, 20, 0).right;
    let bytes_width = du(window, 0, 0, 80, 0).right;
    for (index, (label, width)) in [
        ("N.", number_width),
        ("Downloaded", bytes_width),
        (
            "Info",
            (list_rect.right - number_width - bytes_width - 18).max(80),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mut label = wide(label);
        let mut column: ListColumn = unsafe { std::mem::zeroed() };
        column.mask = 1 | 2 | 4 | 8;
        column.width = width;
        column.text = label.as_mut_ptr();
        column.subitem = index as i32;
        send(
            list,
            LVM + 97,
            index,
            (&mut column as *mut ListColumn) as isize,
        );
    }
    if unsafe { SetWindowSubclass(child(window, 1179), Some(map_proc), 1, pointer as usize) } == 0 {
        return Err("Could not initialize the connection progress map.".into());
    }
    let mut full = Rect::default();
    unsafe {
        GetWindowRect(window, &mut full);
    }
    state.full_height.set(full.bottom - full.top);
    Ok(())
}

fn state_for<'a>(window: Handle) -> Option<&'a DialogState> {
    let pointer = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) } as *const DialogState;
    unsafe { pointer.as_ref() }
}
unsafe extern "system" fn dialog_proc(
    window: Handle,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if message == WM_INITDIALOG {
        unsafe {
            SetWindowLongPtrW(window, GWLP_USERDATA, l_param);
        }
        return 1;
    }
    let Some(state) = state_for(window) else {
        return 0;
    };
    match message {
        WM_TIMER => {
            refresh(state);
            1
        }
        WM_CLOSE => {
            close(state, false);
            1
        }
        WM_DESTROY => {
            unsafe {
                KillTimer(window, 1);
                PostQuitMessage(0);
            }
            1
        }
        WM_COMMAND => {
            match (w_param & 0xffff) as i32 {
                2 => close(state, true),
                1158 => {
                    if let Some(snapshot) = state.snapshot.borrow().as_ref() {
                        let action =
                            if matches!(snapshot.download.status, Status::Paused | Status::Error) {
                                Action::Resume(snapshot.download.id)
                            } else {
                                Action::Pause(snapshot.download.id)
                            };
                        lock(&state.shared).actions.push(action);
                    }
                }
                1178 => toggle_details(state),
                _ => return 0,
            }
            1
        }
        WM_NOTIFY => {
            let notification = unsafe { (l_param as *const Notify).as_ref() };
            if notification.is_some_and(|n| n.id == TAB_ID as usize && n.code == (-551i32) as u32) {
                select_page(state, send(state.tab.get(), TCM + 11, 0, 0).max(0) as usize);
                1
            } else {
                0
            }
        }
        WM_CTLCOLORSTATIC => {
            unsafe {
                SetTextColor(w_param as Handle, color(0, 0, 100));
                SetBkMode(w_param as Handle, 1);
            }
            (unsafe { GetSysColorBrush(15) }) as isize
        }
        _ => 0,
    }
}

unsafe extern "system" fn page_proc(
    window: Handle,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if message == WM_INITDIALOG {
        unsafe {
            SetWindowLongPtrW(window, GWLP_USERDATA, l_param);
        }
        return 1;
    }
    let Some(state) = state_for(window) else {
        return 0;
    };
    match message {
        WM_CTLCOLORDLG | WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            let dc = w_param as Handle;
            let id = if message == WM_CTLCOLORSTATIC {
                unsafe { GetDlgCtrlID(l_param as Handle) }
            } else {
                0
            };
            unsafe {
                SetBkColor(dc, color(255, 255, 255));
                SetBkMode(dc, 1);
                SetTextColor(
                    dc,
                    if id == 1030 {
                        color(0, 0, 255)
                    } else {
                        color(0, 0, 100)
                    },
                );
                GetStockObject(0) as isize // WHITE_BRUSH
            }
        }
        WM_COMMAND => {
            let id = (w_param & 0xffff) as i32;
            let notification = (w_param >> 16) as u16;
            if id == 2 {
                close(state, false);
                return 1;
            }
            if id == 1772 {
                send(state.tab.get(), TCM + 12, 0, 0);
                select_page(state, 0);
                return 1;
            }
            if state.syncing.get() {
                return 0;
            }
            if (notification == 0 && matches!(id, 1707 | 1765 | 1236 | 1227 | 1271))
                || (id == 1764 && notification == 0x0300)
            {
                preferences(state, id);
                return 1;
            }
            0
        }
        _ => 0,
    }
}

fn select_page(state: &DialogState, selected: usize) {
    for (index, page) in state.pages.get().into_iter().enumerate() {
        show(page, index == selected);
    }
}

fn close(state: &DialogState, cancel: bool) {
    let mut shared = lock(&state.shared);
    if cancel {
        if let Some(snapshot) = state.snapshot.borrow().as_ref() {
            if snapshot.download.status.active() || snapshot.download.status == Status::Queued {
                shared.actions.push(Action::Cancel(snapshot.download.id));
            }
        }
    }
    shared.visible = false;
    shared.actions.push(Action::Closed);
    drop(shared);
    show(state.window.get(), false);
    state.visible.set(false);
}

fn toggle_details(state: &DialogState) {
    let details = !state.details.get();
    state.details.set(details);
    let window = state.window.get();
    for id in [1305, 1179, 1177] {
        show(child(window, id), details);
    }
    control_text(
        window,
        1178,
        if details {
            "<< Hide details"
        } else {
            ">> Show details"
        },
    );
    let mut outer = Rect::default();
    let mut client = Rect::default();
    unsafe {
        GetWindowRect(window, &mut outer);
        GetClientRect(window, &mut client);
    }
    let height = if details {
        state.full_height.get()
    } else {
        outer.bottom - outer.top - client.bottom + du(window, 0, 0, 0, 148).bottom
    };
    unsafe {
        SetWindowPos(
            window,
            null_mut(),
            0,
            0,
            outer.right - outer.left,
            height,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

fn preferences(state: &DialogState, changed_id: i32) {
    let Some(mut snapshot) = state.snapshot.borrow().clone() else {
        return;
    };
    let pages = state.pages.get();
    let download = &mut snapshot.download;
    if changed_id == 1765 {
        download.speed_limit_kib = None;
    } else if matches!(changed_id, 1707 | 1764) {
        let enabled = checked(pages[1], 1707);
        let mut buffer = [0u16; 64];
        let length = unsafe {
            GetWindowTextW(
                child(pages[1], 1764),
                buffer.as_mut_ptr(),
                buffer.len() as i32,
            )
        };
        let input = String::from_utf16_lossy(&buffer[..length.max(0) as usize]);
        let value = match input.trim().parse::<u64>() {
            Ok(value) => value.clamp(1, 1_024_000),
            Err(_) if changed_id == 1707 => download
                .speed_limit_kib
                .unwrap_or(snapshot.global_limit)
                .max(512),
            Err(_) => return,
        };
        download.speed_limit_kib = Some(if enabled { value } else { 0 });
    }
    download.notify_on_complete = checked(pages[2], 1236);
    download.open_folder_on_complete = checked(pages[2], 1227);
    download.close_progress_on_complete = checked(pages[2], 1271);
    lock(&state.shared).actions.push(Action::Preferences {
        id: download.id,
        speed_limit_kib: download.speed_limit_kib,
        notify_on_complete: download.notify_on_complete,
        open_folder_on_complete: download.open_folder_on_complete,
        close_progress_on_complete: download.close_progress_on_complete,
    });
    update_preferences(state, &snapshot, false);
    state.snapshot.replace(Some(snapshot));
}

fn preference_key(snapshot: &Snapshot) -> (u64, Option<u64>, u64, bool, bool, bool) {
    let download = &snapshot.download;
    (
        download.id,
        download.speed_limit_kib,
        snapshot.global_limit,
        download.notify_on_complete,
        download.open_folder_on_complete,
        download.close_progress_on_complete,
    )
}
fn update_preferences(state: &DialogState, snapshot: &Snapshot, new_id: bool) {
    state.syncing.set(true);
    let pages = state.pages.get();
    let download = &snapshot.download;
    let effective = download.speed_limit_kib.unwrap_or(snapshot.global_limit);
    check(pages[1], 1707, effective > 0);
    for id in [1764, 1763, 1709] {
        enable(child(pages[1], id), effective > 0);
    }
    if new_id || unsafe { GetFocus() } != child(pages[1], 1764) {
        control_text(
            pages[1],
            1764,
            &if effective == 0 { 512 } else { effective }.to_string(),
        );
    }
    check(pages[2], 1236, download.notify_on_complete);
    check(pages[2], 1227, download.open_folder_on_complete);
    check(pages[2], 1271, download.close_progress_on_complete);
    state.syncing.set(false);
}

fn refresh(state: &DialogState) {
    let (latest, visible, generation, exit) = {
        let shared = lock(&state.shared);
        (
            shared.latest.clone(),
            shared.visible,
            shared.generation,
            shared.exit,
        )
    };
    if exit {
        unsafe {
            DestroyWindow(state.window.get());
        }
        return;
    }
    if !visible {
        if state.visible.replace(false) {
            show(state.window.get(), false);
        }
        return;
    }
    let Some(snapshot) = latest else {
        return;
    };
    let previous = state.snapshot.borrow().clone();
    let new_id = previous.as_ref().map(|s| s.download.id) != Some(snapshot.download.id);
    if previous.as_ref().map(preference_key) != Some(preference_key(&snapshot)) {
        update_preferences(state, &snapshot, new_id);
    }
    state.snapshot.replace(Some(snapshot.clone()));
    update_display(state, &snapshot);
    if !state.visible.replace(true) || state.generation.get() != generation {
        state.generation.set(generation);
        if unsafe { GetWindow(state.window.get(), 4) }.is_null() {
            let mut owner: Handle = null_mut();
            unsafe {
                EnumWindows(Some(find_owner), (&mut owner as *mut Handle) as isize);
            }
            if !owner.is_null() {
                let mut owner_rect = Rect::default();
                let mut window_rect = Rect::default();
                unsafe {
                    SetWindowLongPtrW(state.window.get(), -8, owner as isize);
                    GetWindowRect(owner, &mut owner_rect);
                    GetWindowRect(state.window.get(), &mut window_rect);
                    let width = window_rect.right - window_rect.left;
                    let height = window_rect.bottom - window_rect.top;
                    SetWindowPos(
                        state.window.get(),
                        null_mut(),
                        owner_rect.left + (owner_rect.right - owner_rect.left - width) / 2,
                        owner_rect.top + (owner_rect.bottom - owner_rect.top - height) / 2,
                        width,
                        height,
                        SWP_NOACTIVATE | SWP_NOZORDER,
                    );
                }
            }
        }
        // An owned, ordinary top-level window stays above IDX without being globally topmost.
        show(state.window.get(), true);
        unsafe {
            SetForegroundWindow(state.window.get());
        }
    }
}

fn update_display(state: &DialogState, snapshot: &Snapshot) {
    let window = state.window.get();
    let pages = state.pages.get();
    let download = &snapshot.download;
    let fraction = if download.status == Status::Complete {
        1.0
    } else {
        download.percent()
    };
    text(
        window,
        &format!("{:.0}% {} — IDX", fraction * 100.0, download.name()),
    );
    control_text(pages[0], 1156, &download.url);
    control_text(
        pages[0],
        1030,
        match download.status {
            Status::Queued => "Waiting in queue...",
            Status::Connecting => "Connecting...",
            Status::Downloading => "Receiving data...",
            Status::Paused => "Download paused.",
            Status::Verifying => "Verifying SHA-256 checksum...",
            Status::Complete => "Download complete.",
            Status::Error => &download.error,
        },
    );
    control_text(
        pages[0],
        1154,
        &download
            .total
            .map(bytes_text)
            .unwrap_or_else(|| "(unknown)".into()),
    );
    control_text(
        pages[0],
        1147,
        &format!(
            "{}  ( {:.2} % )",
            bytes_text(download.downloaded),
            fraction * 100.0
        ),
    );
    let rate = if download.status.active() {
        format!("{}/sec", bytes_text(download.speed.max(0.0) as u64))
    } else {
        "0 Bytes/sec".into()
    };
    control_text(pages[0], 1151, &rate);
    control_text(pages[1], 1151, &rate);
    let remaining = if download.status == Status::Complete {
        "0 sec".into()
    } else if download.speed > 0.0 && download.status.active() {
        download
            .total
            .map(|total| {
                time_text(total.saturating_sub(download.downloaded) as f64 / download.speed)
            })
            .unwrap_or_else(|| "(unknown)".into())
    } else {
        "(unknown)".into()
    };
    control_text(pages[0], 1149, &remaining);
    control_text(
        pages[0],
        1175,
        if download.resumable { "Yes" } else { "No" },
    );
    control_text(
        pages[2],
        1759,
        &format!("Save To: {}", download.path.display()),
    );
    control_text(
        window,
        1158,
        if matches!(download.status, Status::Paused | Status::Error) {
            "Resume"
        } else {
            "Pause"
        },
    );
    enable(
        child(window, 1158),
        download.status.active()
            || matches!(
                download.status,
                Status::Queued | Status::Paused | Status::Error
            ),
    );
    control_text(
        window,
        2,
        if matches!(download.status, Status::Complete | Status::Verifying) {
            "Close"
        } else {
            "Cancel"
        },
    );
    send(
        child(window, 1146),
        WM_USER + 2,
        (fraction.clamp(0.0, 1.0) * 10000.0).round() as usize,
        0,
    );
    update_connections(state, download);
    unsafe {
        InvalidateRect(child(window, 1179), null(), 0);
    }
}

fn bytes_text(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.3} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.3} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} Bytes")
    }
}
fn time_text(seconds: f64) -> String {
    let seconds = seconds.max(0.0).ceil() as u64;
    if seconds >= 3600 {
        format!("{} hr {:02} min", seconds / 3600, seconds / 60 % 60)
    } else if seconds >= 60 {
        format!("{} min {:02} sec", seconds / 60, seconds % 60)
    } else {
        format!("{seconds} sec")
    }
}
fn update_connections(state: &DialogState, download: &Download) {
    let list = child(state.window.get(), 1177);
    let row_count = send(list, LVM + 4, 0, 0).max(0) as usize;
    if row_count != download.parts.len() {
        send(list, LVM + 9, 0, 0);
        for index in 0..download.parts.len() {
            let mut number = wide(&(index + 1).to_string());
            let mut item: ListItem = unsafe { std::mem::zeroed() };
            item.mask = 1;
            item.item = index as i32;
            item.text = number.as_mut_ptr();
            send(list, LVM + 77, 0, (&mut item as *mut ListItem) as isize);
        }
    }
    for (index, (received, total)) in download.parts.iter().enumerate() {
        let info = if *total > 0 && received >= total {
            "Complete"
        } else {
            match download.status {
                Status::Downloading if *received > 0 => "Receiving data...",
                Status::Downloading | Status::Connecting => "Waiting for data...",
                Status::Queued => "Queued",
                Status::Paused => "Paused",
                Status::Error => "Stopped",
                Status::Verifying | Status::Complete => "Complete",
            }
        };
        for (subitem, value) in [(1, bytes_text(*received)), (2, info.into())] {
            let mut value = wide(&value);
            let mut item: ListItem = unsafe { std::mem::zeroed() };
            item.subitem = subitem;
            item.text = value.as_mut_ptr();
            send(
                list,
                LVM + 116,
                index,
                (&mut item as *mut ListItem) as isize,
            );
        }
    }
}

const fn color(red: u8, green: u8, blue: u8) -> u32 {
    red as u32 | (green as u32) << 8 | (blue as u32) << 16
}
fn paint_rectangle(dc: Handle, rect: Rect, color: u32) {
    let brush = unsafe { CreateSolidBrush(color) };
    unsafe {
        FillRect(dc, &rect, brush);
        DeleteObject(brush);
    }
}
unsafe extern "system" fn map_proc(
    window: Handle,
    message: u32,
    w_param: usize,
    l_param: isize,
    id: usize,
    reference: usize,
) -> isize {
    if message == WM_NCDESTROY {
        unsafe {
            RemoveWindowSubclass(window, Some(map_proc), id);
        }
    }
    if message != WM_PAINT {
        return unsafe { DefSubclassProc(window, message, w_param, l_param) };
    }
    let Some(state) = (unsafe { (reference as *const DialogState).as_ref() }) else {
        return 0;
    };
    let mut paint: Paint = unsafe { std::mem::zeroed() };
    let dc = unsafe { BeginPaint(window, &mut paint) };
    let mut rect = Rect::default();
    unsafe {
        GetClientRect(window, &mut rect);
    }
    paint_rectangle(dc, rect, color(244, 244, 248));
    let snapshot = state.snapshot.borrow();
    if let Some(snapshot) = snapshot.as_ref() {
        let total = snapshot
            .download
            .parts
            .iter()
            .map(|(_, total)| *total as f64)
            .sum::<f64>();
        if total > 0.0 {
            let width = (rect.right - 2).max(0) as f64;
            let height = (rect.bottom - 2).max(0);
            let colors = [
                color(147, 214, 237),
                color(94, 181, 225),
                color(53, 139, 207),
                color(49, 105, 175),
                color(84, 143, 184),
                color(73, 191, 129),
            ];
            let mut position = 0f64;
            for (received, length) in &snapshot.download.parts {
                let left = 1 + (width * position / total).round() as i32;
                let right = 1
                    + (width * (position + (*received).min(*length) as f64) / total).round() as i32;
                for (index, shade) in colors.into_iter().enumerate() {
                    let top = 1 + height * index as i32 / colors.len() as i32;
                    let bottom = 1 + height * (index + 1) as i32 / colors.len() as i32;
                    if right > left {
                        paint_rectangle(
                            dc,
                            Rect {
                                left,
                                right,
                                top,
                                bottom,
                            },
                            shade,
                        );
                    }
                }
                paint_rectangle(
                    dc,
                    Rect {
                        left,
                        right: left + 1,
                        top: 1,
                        bottom: rect.bottom - 1,
                    },
                    color(234, 99, 103),
                );
                position += *length as f64;
            }
        }
    }
    let border = unsafe { CreateSolidBrush(color(119, 120, 150)) };
    unsafe {
        FrameRect(dc, &rect, border);
        DeleteObject(border);
        EndPaint(window, &paint);
    }
    0
}
