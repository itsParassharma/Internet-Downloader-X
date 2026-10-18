//! Stable column identity, visibility, order and DPI-independent widths.
//! Used by the Rust handler for original Columns dialog 209. UI behavior follows
//! CEditColumnsDlg. Original definitions are reproduced in ORIGINAL-COLUMNS.json.

/// (stable subitem ID, visible, width at 96 DPI), in display order.
pub type ColumnPreferences = Vec<(usize, bool, u32)>;

/// Stable IDX IDs retain compatibility with preferences saved by the ten-column
/// revision. Display order follows original IDs 0..11; Date Added is IDX ID 8.
pub const DEFAULTS: [(usize, bool, u32); 12] = [
    (0, true, 150),
    (8, false, 65),
    (1, true, 20),
    (2, true, 75),
    (3, true, 65),
    (4, true, 75),
    (5, true, 95),
    (6, true, 65),
    (7, true, 200),
    (9, false, 200),
    (10, false, 200),
    (11, false, 200),
];

/// Labels from the original CString initializers, indexed by stable IDX ID.
pub const TITLES: [&str; 12] = [
    "File Name",
    "Q",
    "Size",
    "Status",
    "Time left",
    "Transfer rate",
    "Last Try Date",
    "Description",
    "Date Added",
    "Save To",
    "Referer",
    "Parent web page",
];

pub fn normalize(
    saved: &[(usize, bool, u32)],
    defaults: &[(usize, bool, u32)],
) -> ColumnPreferences {
    let mut result = Vec::with_capacity(defaults.len());
    for &(id, visible, width) in saved {
        if defaults.iter().any(|column| column.0 == id)
            && !result.iter().any(|&(other, _, _)| other == id)
        {
            result.push((id, visible, width.clamp(1, i32::MAX as u32)));
        }
    }
    for &(id, visible, width) in defaults {
        if !result.iter().any(|&(other, _, _)| other == id) {
            result.push((id, visible, width.clamp(1, i32::MAX as u32)));
        }
    }
    result
}

pub fn move_selected(columns: &mut [(usize, bool, u32)], row: usize, up: bool) -> usize {
    if row >= columns.len() {
        return row;
    }
    let next = if up {
        row.saturating_sub(1)
    } else {
        (row + 1).min(columns.len() - 1)
    };
    columns.swap(row, next);
    next
}

/// Original Show/Hide handlers 0x4a9fb0/0x4aa000 change the state image
/// without discarding the saved width or the logical column's identity.
pub fn set_visible(columns: &mut [(usize, bool, u32)], row: usize, visible: bool) {
    if let Some(column) = columns.get_mut(row) {
        column.1 = visible;
    }
}

/// Original 0x4aa230 stores only a positive parsed width for the selected column.
pub fn set_width(columns: &mut [(usize, bool, u32)], row: usize, text: &str) -> bool {
    // 0x63e251 delegates to Windows _atol: leading ASCII whitespace, an
    // optional plus sign and the decimal prefix are accepted. 0x4aa230 only
    // stores a positive result. Overflow has no portable original behavior;
    // preserve the previous width for out-of-range input.
    let text = text.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let text = text.strip_prefix('+').unwrap_or(text);
    let mut width = 0u32;
    for byte in text.bytes().take_while(u8::is_ascii_digit) {
        let Some(next) = width
            .checked_mul(10)
            .and_then(|value| value.checked_add(u32::from(byte - b'0')))
        else {
            return false;
        };
        if next > i32::MAX as u32 {
            return false;
        }
        width = next;
    }
    if width == 0 {
        return false;
    }
    let Some(column) = columns.get_mut(row) else {
        return false;
    };
    column.2 = width;
    true
}

/// MulDiv-based conversion used by original 0x4a9b10 and 0x4aa050.
pub fn to_pixels(logical: u32, dpi: u32) -> u32 {
    ((u64::from(logical) * u64::from(dpi.max(96)) + 48) / 96).min(i32::MAX as u64) as u32
}
pub fn to_logical(pixels: u32, dpi: u32) -> u32 {
    let dpi = u64::from(dpi.max(96));
    ((u64::from(pixels) * 96 + dpi / 2) / dpi).clamp(1, i32::MAX as u64) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ten_column_preferences_survive_upgrade_and_new_columns_start_hidden() {
        let saved = vec![
            (8, true, 140),
            (0, false, 230),
            (1, true, 28),
            (2, true, 85),
            (3, true, 90),
            (4, true, 80),
            (5, true, 100),
            (6, true, 140),
            (7, true, 160),
            (9, true, 260),
        ];
        let upgraded = normalize(&saved, &DEFAULTS);
        assert_eq!(&upgraded[..saved.len()], &saved);
        assert_eq!(&upgraded[10..], &[(10, false, 200), (11, false, 200)]);
        assert_eq!(normalize(&upgraded, &DEFAULTS), upgraded);
    }
    #[test]
    fn missing_columns_keep_reference_order_and_visibility() {
        let restored = normalize(&[(11, true, 321)], &DEFAULTS);
        assert_eq!(restored[0], (11, true, 321));
        assert_eq!(&restored[1..], &DEFAULTS[..11]);
        assert_eq!(normalize(&[], &DEFAULTS), DEFAULTS);
    }
    #[test]
    fn legacy_defaults_and_new_columns_keep_stable_ids() {
        assert_eq!(
            normalize(&[], &[(0, true, 150), (1, true, 20)]),
            vec![(0, true, 150), (1, true, 20)]
        );
        assert_eq!(
            normalize(
                &[(1, false, 35)],
                &[(0, true, 150), (1, true, 20), (2, true, 75)]
            ),
            vec![(1, false, 35), (0, true, 150), (2, true, 75)]
        );
    }
    #[test]
    fn invalid_saved_order_is_repaired_without_losing_valid_preferences() {
        assert_eq!(
            normalize(
                &[(1, false, 45), (1, true, 80), (99, true, 55), (0, true, 0)],
                &[(0, true, 150), (1, true, 20), (2, true, 75)]
            ),
            vec![(1, false, 45), (0, true, 1), (2, true, 75)]
        );
    }
    #[test]
    fn moving_then_hiding_keeps_width_attached_to_identity() {
        let mut columns = normalize(&[], &[(0, true, 150), (1, true, 20), (2, true, 75)]);
        assert_eq!(move_selected(&mut columns, 2, true), 1);
        set_visible(&mut columns, 1, false);
        assert_eq!(columns, vec![(0, true, 150), (2, false, 75), (1, true, 20)]);
        set_visible(&mut columns, 1, true);
        assert_eq!(columns[1], (2, true, 75));
    }
    #[test]
    fn movement_stops_at_both_boundaries() {
        let mut columns = normalize(&[], &[(0, true, 150), (1, true, 20)]);
        assert_eq!(move_selected(&mut columns, 0, true), 0);
        assert_eq!(move_selected(&mut columns, 1, false), 1);
        assert_eq!(move_selected(&mut [], 0, true), 0);
        assert_eq!(columns, normalize(&[], &[(0, true, 150), (1, true, 20)]));
    }
    #[test]
    fn width_edit_rejects_invalid_input_without_corrupting_previous_width() {
        let mut columns = normalize(&[], &[(0, true, 150)]);
        for invalid in ["", "0", "-1", "abc", "2147483648", "999999999999999999"] {
            assert!(!set_width(&mut columns, 0, invalid));
            assert_eq!(columns[0].2, 150);
        }
        assert!(set_width(&mut columns, 0, " 240 "));
        assert_eq!(columns[0].2, 240);
        assert!(set_width(&mut columns, 0, "\t+360px"));
        assert_eq!(columns[0].2, 360);
    }
    #[test]
    fn dpi_widths_use_muldiv_rounding() {
        for dpi in [96, 120, 144, 192] {
            assert_eq!(to_logical(to_pixels(200, dpi), dpi), 200);
        }
        assert_eq!(to_pixels(75, 144), 113);
        assert_eq!(to_logical(300, 144), 200);
    }
}
