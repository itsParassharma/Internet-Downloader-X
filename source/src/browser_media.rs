//! Validation shared by the native host and durable browser inbox.
//! Browser-supplied names are filenames, never destination paths.
pub fn validate_filename(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Ok(());
    }
    if name.len() > 240
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || name.ends_with([' ', '.'])
        || name.starts_with(' ')
    {
        return Err("Invalid browser filename.");
    }
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
    {
        return Err("Reserved browser filename.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_names_cannot_choose_paths_or_windows_devices() {
        for name in [
            "../video.mp4",
            "C:\\video.mp4",
            "video.mp4:secret",
            "CON.mp4",
            "lpt1",
            "file.",
            "file ",
            "a\n.mp4",
        ] {
            assert!(validate_filename(name).is_err(), "{name}");
        }
        for name in ["", "Video 1080p.mp4", "日本語.webm", "concert.mp3"] {
            assert!(validate_filename(name).is_ok(), "{name}");
        }
    }
}
