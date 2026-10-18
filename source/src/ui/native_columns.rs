//! Rust handlers for extracted CEditColumnsDlg, resource 209.
//! Original routine addresses are recorded at each translated behavior. The
//! template provides the unchanged geometry, control IDs, font and captions.
#![allow(unsafe_op_in_unsafe_fn)]

use crate::columns::{self, ColumnPreferences};
use std::{
    ffi::c_void,
    ptr::{null, null_mut},
};

type Handle = *mut c_void;
type Proc = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;
const LVM: u32 = 0x1000;
const USERDATA: i32 = -21;

#[repr(C)]
#[derive(Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
#[repr(C)]
struct Notify {
    hwnd: Handle,
    id: usize,
    code: u32,
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

#[link(name = "user32")]
unsafe extern "system" {
    fn DialogBoxIndirectParamW(
        instance: Handle,
        template: *const c_void,
        owner: Handle,
        proc: Option<Proc>,
        param: isize,
    ) -> isize;
    fn EndDialog(hwnd: Handle, result: isize) -> i32;
    fn GetDlgItem(hwnd: Handle, id: i32) -> Handle;
    fn SendMessageW(hwnd: Handle, msg: u32, w: usize, l: isize) -> isize;
    fn SetWindowLongPtrW(hwnd: Handle, index: i32, value: isize) -> isize;
    fn GetWindowLongPtrW(hwnd: Handle, index: i32) -> isize;
    fn SetWindowTextW(hwnd: Handle, text: *const u16) -> i32;
    fn GetWindowTextW(hwnd: Handle, text: *mut u16, len: i32) -> i32;
    fn EnableWindow(hwnd: Handle, enable: i32) -> i32;
    fn SetTimer(hwnd: Handle, id: usize, ms: u32, callback: *const c_void) -> usize;
    fn KillTimer(hwnd: Handle, id: usize) -> i32;
    fn GetWindowRect(hwnd: Handle, rect: *mut Rect) -> i32;
    fn SetWindowPos(
        hwnd: Handle,
        after: Handle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    fn LoadIconW(instance: Handle, name: *const u16) -> Handle;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> Handle;
    fn GetLastError() -> u32;
}

struct State<'a> {
    hwnd: Handle,
    owner: Handle,
    columns: ColumnPreferences,
    defaults: &'a [(usize, bool, u32)],
    titles: &'a [&'a str],
    dpi: u32,
    selected: Option<usize>,
    busy: bool,
    accepted: bool,
    tick: &'a mut dyn FnMut(),
}

pub(super) fn show(
    owner: Handle,
    saved: &[(usize, bool, u32)],
    defaults: &[(usize, bool, u32)],
    titles: &[&str],
    dpi: u32,
    mut tick: impl FnMut(),
) -> Result<Option<ColumnPreferences>, String> {
    if defaults.len() != titles.len() || defaults.iter().any(|column| column.0 >= titles.len()) {
        return Err("Column definitions do not match.".into());
    }
    let mut state = State {
        hwnd: null_mut(),
        owner,
        columns: columns::normalize(saved, defaults),
        defaults,
        titles,
        dpi,
        selected: None,
        busy: false,
        accepted: false,
        tick: &mut tick,
    };
    // Constructor 0x4a9b70 uses the same resource and parent relationship.
    let template = super::native_resources::top_level(209);
    let result = unsafe {
        DialogBoxIndirectParamW(
            GetModuleHandleW(null()),
            template.as_ptr().cast(),
            owner,
            Some(proc),
            (&mut state as *mut State<'_>) as isize,
        )
    };
    if result == -1 {
        return Err(format!(
            "Could not open Columns (Windows error {}).",
            unsafe { GetLastError() }
        ));
    }
    Ok(state.accepted.then_some(state.columns))
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
unsafe fn set(hwnd: Handle, id: i32, text: &str) {
    SetWindowTextW(GetDlgItem(hwnd, id), wide(text).as_ptr());
}
unsafe fn enable(hwnd: Handle, id: i32, on: bool) {
    EnableWindow(GetDlgItem(hwnd, id), i32::from(on));
}
unsafe fn selected(list: Handle) -> Option<usize> {
    let index = SendMessageW(list, LVM + 12, usize::MAX, 2);
    (index >= 0).then_some(index as usize)
}

// Initialization 0x4aa7c0: checkboxes, ordered logical IDs, cached widths,
// and disabled action buttons until a row is selected.
unsafe fn init(state: &mut State<'_>) {
    let list = GetDlgItem(state.hwnd, 1023);
    let column = ListColumn {
        mask: 2,
        width: columns::to_pixels(180, state.dpi) as i32,
        ..Default::default()
    };
    SendMessageW(list, LVM + 97, 0, (&column as *const ListColumn) as isize);
    SendMessageW(list, LVM + 54, 0, 0x24);
    SendMessageW(GetDlgItem(state.hwnd, 1031), 0xc5, 31, 0);
    let icon = LoadIconW(GetModuleHandleW(null()), 101usize as *const u16);
    if !icon.is_null() {
        SendMessageW(state.hwnd, 0x80, 0, icon as isize);
        SendMessageW(state.hwnd, 0x80, 1, icon as isize);
    }
    if !state.owner.is_null() {
        let (mut owner, mut dialog) = (Rect::default(), Rect::default());
        GetWindowRect(state.owner, &mut owner);
        GetWindowRect(state.hwnd, &mut dialog);
        SetWindowPos(
            state.hwnd,
            null_mut(),
            owner.left + (owner.right - owner.left - dialog.right + dialog.left) / 2,
            owner.top + (owner.bottom - owner.top - dialog.bottom + dialog.top) / 2,
            0,
            0,
            0x15,
        );
    }
    rebuild(state);
    SetTimer(state.hwnd, 1, 150, null());
}

unsafe fn rebuild(state: &mut State<'_>) {
    let list = GetDlgItem(state.hwnd, 1023);
    SendMessageW(list, LVM + 9, 0, 0);
    for (row, &(id, visible, _)) in state.columns.iter().enumerate() {
        let mut text = wide(state.titles[id]);
        let item = ListItem {
            mask: 5,
            item: row as i32,
            text: text.as_mut_ptr(),
            param: id as isize,
            ..Default::default()
        };
        SendMessageW(list, LVM + 77, 0, (&item as *const ListItem) as isize);
        let item = ListItem {
            state: if visible { 0x2000 } else { 0x1000 },
            state_mask: 0xf000,
            ..Default::default()
        };
        SendMessageW(list, LVM + 43, row, (&item as *const ListItem) as isize);
    }
    if let Some(row) = state.selected.filter(|r| *r < state.columns.len()) {
        let item = ListItem {
            state: 3,
            state_mask: 3,
            ..Default::default()
        };
        SendMessageW(list, LVM + 43, row, (&item as *const ListItem) as isize);
        SendMessageW(list, LVM + 19, row, 0);
    }
    refresh_controls(state);
}

unsafe fn read_checks(state: &mut State<'_>) {
    let list = GetDlgItem(state.hwnd, 1023);
    for row in 0..state.columns.len() {
        let checked = (SendMessageW(list, LVM + 44, row, 0xf000) >> 12) == 2;
        columns::set_visible(&mut state.columns, row, checked);
    }
}

// Selection handler 0x4aa430: Show/Hide reflect the checkbox, Move Up/Down
// stop at the first/last item, and the width belongs to the logical ID.
unsafe fn refresh_controls(state: &mut State<'_>) {
    state.selected = selected(GetDlgItem(state.hwnd, 1023));
    let entry = state.selected.and_then(|row| state.columns.get(row));
    enable(state.hwnd, 1340, entry.is_some_and(|column| !column.1));
    enable(state.hwnd, 1048, entry.is_some_and(|column| column.1));
    enable(state.hwnd, 1160, state.selected.is_some_and(|row| row > 0));
    enable(
        state.hwnd,
        1083,
        state
            .selected
            .is_some_and(|row| row + 1 < state.columns.len()),
    );
    enable(state.hwnd, 1031, entry.is_some());
    set(
        state.hwnd,
        1031,
        &entry.map(|column| column.2.to_string()).unwrap_or_default(),
    );
}

unsafe fn edit_width(state: &mut State<'_>) {
    if let Some(row) = state.selected {
        let mut text = [0u16; 32];
        let len = GetWindowTextW(
            GetDlgItem(state.hwnd, 1031),
            text.as_mut_ptr(),
            text.len() as i32,
        )
        .max(0) as usize;
        columns::set_width(
            &mut state.columns,
            row,
            &String::from_utf16_lossy(&text[..len]),
        );
    }
}

unsafe fn command(state: &mut State<'_>, id: usize) {
    match id {
        1 => {
            // Save handler 0x4aa050; parent applies widths/order to its list.
            edit_width(state);
            read_checks(state);
            state.accepted = true;
            EndDialog(state.hwnd, 1);
        }
        2 => {
            EndDialog(state.hwnd, 0);
        }
        1340 | 1048 => {
            read_checks(state);
            if let Some(row) = state.selected {
                columns::set_visible(&mut state.columns, row, id == 1340);
                rebuild(state);
            }
        }
        1160 | 1083 => {
            read_checks(state);
            if let Some(row) = state.selected {
                state.selected = Some(columns::move_selected(&mut state.columns, row, id == 1160));
                rebuild(state);
            }
        }
        1053 => {
            // Reset workflow 0x4aab20: all twelve original columns, with their
            // recovered order, visibility and 96-DPI widths.
            state.columns = columns::normalize(&[], state.defaults);
            state.selected = None;
            rebuild(state);
        }
        _ => {}
    }
}

unsafe extern "system" fn proc(hwnd: Handle, msg: u32, w: usize, l: isize) -> isize {
    if msg == 0x110 {
        SetWindowLongPtrW(hwnd, USERDATA, l);
        let state = &mut *(l as *mut State<'_>);
        state.hwnd = hwnd;
        state.busy = true;
        init(state);
        state.busy = false;
        return 1;
    }
    let ptr = GetWindowLongPtrW(hwnd, USERDATA) as *mut State<'_>;
    if ptr.is_null() || (*ptr).busy {
        return 0;
    }
    let state = &mut *ptr;
    state.busy = true;
    let handled = match msg {
        0x111 => {
            let id = w & 0xffff;
            let code = w >> 16;
            if id == 1031 && code == 0x300 {
                edit_width(state);
            } else if code == 0 {
                command(state, id);
            }
            1
        }
        0x4e if l != 0 => {
            let notice = &*(l as *const Notify);
            if notice.id == 1023 && notice.code == (-101i32) as u32 {
                read_checks(state);
                refresh_controls(state);
            }
            1
        }
        0x113 => {
            (state.tick)();
            1
        }
        0x10 => {
            EndDialog(hwnd, 0);
            1
        }
        2 => {
            KillTimer(hwnd, 1);
            0
        }
        _ => 0,
    };
    state.busy = false;
    handled
}
