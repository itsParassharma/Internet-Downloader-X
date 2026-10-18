//! Original filter manager 244 and name/mask editor 245.
use super::*;
use crate::grabber::filters::{NamedFilter, validate_named};

pub(super) struct Edited {
    pub filters: Vec<NamedFilter>,
    pub origins: Vec<Option<usize>>,
}
struct Manager {
    app: *mut IdxApp,
    context: egui::Context,
    draft: Edited,
    accepted: bool,
    busy: bool,
}
pub(super) unsafe fn edit(
    owner: Handle,
    app: *mut IdxApp,
    filters: &[NamedFilter],
) -> Result<Option<Edited>, String> {
    let mut s = Manager {
        app,
        context: egui::Context::default(),
        draft: Edited {
            filters: filters.to_vec(),
            origins: (0..filters.len()).map(Some).collect(),
        },
        accepted: false,
        busy: true,
    };
    let template = super::super::native_resources::top_level(244);
    if DialogBoxIndirectParamW(
        GetModuleHandleW(null()),
        template.as_ptr().cast(),
        owner,
        Some(manager_proc),
        (&mut s as *mut Manager) as isize,
    ) == -1
    {
        return Err("Could not open the filter manager.".into());
    }
    Ok(s.accepted.then_some(s.draft))
}
unsafe fn refresh(hwnd: Handle, s: &Manager, selected: Option<usize>) {
    let list = GetDlgItem(hwnd, 1023);
    SendMessageW(list, LVM + 9, 0, 0);
    for (i, f) in s.draft.filters.iter().enumerate() {
        row(list, i, &f.name, i as isize);
        cell(list, i, 1, &f.mask);
    }
    if let Some(index) = selected.filter(|i| *i < s.draft.filters.len()) {
        select_row(list, index);
    }
    buttons(hwnd);
}
unsafe fn buttons(hwnd: Handle) {
    let selected = selected_row(GetDlgItem(hwnd, 1023)).is_some();
    enabled(hwnd, 1424, selected);
    enabled(hwnd, 1425, selected);
}
unsafe fn change(hwnd: Handle, s: &mut Manager, adding: bool) {
    let editing = if adding {
        None
    } else {
        selected_row(GetDlgItem(hwnd, 1023))
    };
    if !adding && editing.is_none() {
        return;
    }
    match editor(hwnd, s.app, &s.draft.filters, editing) {
        Ok(Some(filter)) => {
            let selected = if let Some(i) = editing {
                s.draft.filters[i] = filter;
                i
            } else {
                s.draft.filters.push(filter);
                s.draft.origins.push(None);
                s.draft.filters.len() - 1
            };
            refresh(hwnd, s, Some(selected));
        }
        Ok(None) => {}
        Err(error) => info(hwnd, &error),
    }
}
unsafe extern "system" fn manager_proc(hwnd: Handle, msg: u32, w: usize, l: isize) -> isize {
    if msg == WM_INIT {
        SetWindowLongPtrW(hwnd, USERDATA, l);
        let s = &mut *(l as *mut Manager);
        let width = rect(hwnd, 0, 0, 261, 0).right;
        columns(
            GetDlgItem(hwnd, 1023),
            &[("Name", width / 3), ("Mask", width * 2 / 3 - 8)],
        );
        refresh(hwnd, s, None);
        icon(hwnd);
        center(hwnd, GetParent(hwnd));
        SetTimer(hwnd, 1, 150, null());
        s.busy = false;
        return 1;
    }
    let ptr = GetWindowLongPtrW(hwnd, USERDATA) as *mut Manager;
    if ptr.is_null() || (*ptr).busy {
        return 0;
    }
    let s = &mut *ptr;
    s.busy = true;
    match msg {
        WM_COMMAND if w >> 16 == 0 => match w & 0xffff {
            1423 => change(hwnd, s, true),
            1424 => change(hwnd, s, false),
            1425 => {
                if let Some(i) = selected_row(GetDlgItem(hwnd, 1023)) {
                    s.draft.filters.remove(i);
                    s.draft.origins.remove(i);
                    refresh(
                        hwnd,
                        s,
                        Some(i.min(s.draft.filters.len().saturating_sub(1))),
                    );
                }
            }
            1 => {
                s.accepted = true;
                EndDialog(hwnd, 1);
            }
            2 => {
                EndDialog(hwnd, 0);
            }
            _ => {}
        },
        WM_NOTIFY if l != 0 => {
            let n = &*(l as *const Notify);
            if n.id == 1023 {
                buttons(hwnd);
                if n.code == (-3i32) as u32 {
                    change(hwnd, s, false);
                }
            }
        }
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
struct Editor {
    app: *mut IdxApp,
    context: egui::Context,
    filters: Vec<NamedFilter>,
    editing: Option<usize>,
    result: Option<NamedFilter>,
    busy: bool,
}
unsafe fn editor(
    owner: Handle,
    app: *mut IdxApp,
    filters: &[NamedFilter],
    editing: Option<usize>,
) -> Result<Option<NamedFilter>, String> {
    let mut s = Editor {
        app,
        context: egui::Context::default(),
        filters: filters.to_vec(),
        editing,
        result: None,
        busy: true,
    };
    let template = super::super::native_resources::top_level(245);
    if DialogBoxIndirectParamW(
        GetModuleHandleW(null()),
        template.as_ptr().cast(),
        owner,
        Some(editor_proc),
        (&mut s as *mut Editor) as isize,
    ) == -1
    {
        return Err("Could not open the filter editor.".into());
    }
    Ok(s.result)
}
unsafe extern "system" fn editor_proc(hwnd: Handle, msg: u32, w: usize, l: isize) -> isize {
    if msg == WM_INIT {
        SetWindowLongPtrW(hwnd, USERDATA, l);
        let s = &mut *(l as *mut Editor);
        if let Some(f) = s.editing.and_then(|i| s.filters.get(i)) {
            set(hwnd, 1428, &f.name);
            set(hwnd, 1040, &f.mask);
        }
        SendMessageW(GetDlgItem(hwnd, 1428), 0xc5, 128, 0);
        SendMessageW(GetDlgItem(hwnd, 1040), 0xc5, 8192, 0);
        icon(hwnd);
        center(hwnd, GetParent(hwnd));
        SetTimer(hwnd, 1, 150, null());
        s.busy = false;
        return 1;
    }
    let ptr = GetWindowLongPtrW(hwnd, USERDATA) as *mut Editor;
    if ptr.is_null() || (*ptr).busy {
        return 0;
    }
    let s = &mut *ptr;
    s.busy = true;
    match msg {
        WM_COMMAND if w >> 16 == 0 => match w & 0xffff {
            1 => match validate_named(
                NamedFilter {
                    name: get(hwnd, 1428),
                    mask: get(hwnd, 1040),
                },
                &s.filters,
                s.editing,
            ) {
                Ok(filter) => {
                    s.result = Some(filter);
                    EndDialog(hwnd, 1);
                }
                Err(error) => info(hwnd, &error),
            },
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
