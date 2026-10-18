//! Browser connection is configured by IDX after installation.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Setup {
    pub offered: bool,
    pub extension_ids: Vec<String>,
}
pub fn load(data: &Path) -> Setup {
    std::fs::read(data.join("browser-setup.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
fn save(data: &Path, setup: &Setup) -> Result<(), String> {
    std::fs::create_dir_all(data).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(setup).map_err(|e| e.to_string())?;
    let temporary = data.join("browser-setup.json.tmp");
    std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(temporary, data.join("browser-setup.json")).map_err(|e| e.to_string())
}
pub fn mark_offered(data: &Path) -> Result<(), String> {
    let mut setup = load(data);
    setup.offered = true;
    save(data, &setup)
}
pub fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| (b'a'..=b'p').contains(&b))
}
pub fn bundled_id() -> String {
    let hash = Sha256::digest(include_bytes!("../assets/extension-public-key.der"));
    hash[..16]
        .iter()
        .flat_map(|b| [char::from(b'a' + (b >> 4)), char::from(b'a' + (b & 15))])
        .collect()
}
pub fn connect_bundled(data: &Path, executable: &Path) -> Result<(), String> {
    connect(data, executable, &bundled_id())
}
pub fn extension_folder(executable: &Path) -> Option<PathBuf> {
    executable
        .ancestors()
        .skip(1)
        .take(4)
        .map(|p| p.join("browser-extension/chromium"))
        .find(|p| p.join("manifest.json").is_file())
}
fn manifest(host: &Path, ids: &[String]) -> serde_json::Value {
    serde_json::json!({
        "name":"org.idx.downloader", "description":"IDX browser bridge",
        "path":host, "type":"stdio",
        "allowed_origins":ids.iter().map(|id| format!("chrome-extension://{id}/")).collect::<Vec<_>>()
    })
}

pub fn connect(data: &Path, executable: &Path, id: &str) -> Result<(), String> {
    let id = id.trim();
    if !valid_id(id) {
        return Err(
            "Enter the 32-letter ID shown for IDX on your browser's Extensions page.".into(),
        );
    }
    let folder = executable
        .parent()
        .ok_or("Cannot locate the IDX application folder.")?;
    let host = folder.join("idx-native-host.exe");
    if !host.is_file() {
        return Err("idx-native-host.exe must be beside idx.exe. Reinstall IDX or extract the complete portable ZIP.".into());
    }
    let mut setup = load(data);
    setup.extension_ids.retain(|id| valid_id(id));
    // Preserve connections created by an earlier version in this installation.
    for browser in ["chrome", "edge"] {
        let file = folder.join(format!("browser-extension/native-hosts/{browser}.json"));
        if let Some(old) = std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        {
            let old_host = old["path"].as_str().map(|p| file.parent().unwrap().join(p));
            if old["name"] == "org.idx.downloader"
                && old_host.and_then(|p| std::fs::canonicalize(p).ok())
                    == std::fs::canonicalize(&host).ok()
            {
                if let Some(origins) = old["allowed_origins"].as_array() {
                    for origin in origins {
                        if let Some(id) = origin
                            .as_str()
                            .and_then(|s| s.strip_prefix("chrome-extension://"))
                            .and_then(|s| s.strip_suffix('/'))
                            .filter(|s| valid_id(s))
                        {
                            if !setup.extension_ids.iter().any(|existing| existing == id) {
                                setup.extension_ids.push(id.into());
                            }
                        }
                    }
                }
            }
        }
    }
    if !setup.extension_ids.iter().any(|existing| existing == id) {
        setup.extension_ids.push(id.into());
    }
    let contents = serde_json::to_vec_pretty(&manifest(&host, &setup.extension_ids))
        .map_err(|e| e.to_string())?;
    #[cfg(windows)]
    windows::register(&folder.join("browser-extension/native-hosts"), &contents)?;
    #[cfg(not(windows))]
    {
        let _ = contents;
        return Err("Browser connection setup is currently available on Windows.".into());
    }
    #[cfg(windows)]
    save(data, &setup)
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{
        ffi::c_void,
        ptr::{null, null_mut},
    };
    type Key = *mut c_void;
    const CURRENT_USER: Key = 0x80000001u32 as i32 as isize as Key;
    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegOpenKeyExW(
            root: Key,
            path: *const u16,
            options: u32,
            access: u32,
            key: *mut Key,
        ) -> i32;
        fn RegCreateKeyExW(
            root: Key,
            path: *const u16,
            reserved: u32,
            class: *mut u16,
            options: u32,
            access: u32,
            security: *const c_void,
            key: *mut Key,
            disposition: *mut u32,
        ) -> i32;
        fn RegQueryValueExW(
            key: Key,
            name: *const u16,
            reserved: *mut u32,
            kind: *mut u32,
            data: *mut u8,
            size: *mut u32,
        ) -> i32;
        fn RegSetValueExW(
            key: Key,
            name: *const u16,
            reserved: u32,
            kind: u32,
            data: *const u8,
            size: u32,
        ) -> i32;
        fn RegDeleteValueW(key: Key, name: *const u16) -> i32;
        fn RegCloseKey(key: Key) -> i32;
    }
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }
    fn read(path: &str, view: u32) -> Result<Option<String>, String> {
        unsafe {
            let mut key = null_mut();
            let result = RegOpenKeyExW(CURRENT_USER, wide(path).as_ptr(), 0, 1 | view, &mut key);
            if result == 2 {
                return Ok(None);
            }
            if result != 0 {
                return Err(format!(
                    "Cannot read browser registration (Windows error {result})."
                ));
            }
            let mut kind = 0;
            let mut size = 0;
            let result =
                RegQueryValueExW(key, null(), null_mut(), &mut kind, null_mut(), &mut size);
            if result == 2 {
                RegCloseKey(key);
                return Ok(None);
            }
            if result != 0 || kind != 1 || size > 65536 {
                RegCloseKey(key);
                return Err("Cannot read the existing browser connection.".into());
            }
            let mut value = vec![0u16; (size as usize).div_ceil(2)];
            let result = RegQueryValueExW(
                key,
                null(),
                null_mut(),
                &mut kind,
                value.as_mut_ptr().cast(),
                &mut size,
            );
            RegCloseKey(key);
            if result != 0 {
                return Err(format!(
                    "Cannot read browser registration (Windows error {result})."
                ));
            }
            while value.last() == Some(&0) {
                value.pop();
            }
            Ok(Some(String::from_utf16_lossy(&value)))
        }
    }
    fn write(path: &str, view: u32, value: Option<&str>) -> Result<(), String> {
        unsafe {
            let mut key = null_mut();
            let result = RegCreateKeyExW(
                CURRENT_USER,
                wide(path).as_ptr(),
                0,
                null_mut(),
                0,
                2 | view,
                null(),
                &mut key,
                null_mut(),
            );
            if result != 0 {
                return Err(format!(
                    "Cannot register the browser connection (Windows error {result})."
                ));
            }
            let result = if let Some(value) = value {
                let value = wide(value);
                RegSetValueExW(
                    key,
                    null(),
                    0,
                    1,
                    value.as_ptr().cast(),
                    (value.len() * 2) as u32,
                )
            } else {
                RegDeleteValueW(key, null())
            };
            RegCloseKey(key);
            if result != 0 && !(value.is_none() && result == 2) {
                return Err(format!(
                    "Cannot save the browser connection (Windows error {result})."
                ));
            }
            Ok(())
        }
    }
    pub fn register(folder: &Path, contents: &[u8]) -> Result<(), String> {
        let mut values = Vec::new();
        let mut files = Vec::new();
        for (vendor, name) in [("Google\\Chrome", "chrome"), ("Microsoft\\Edge", "edge")] {
            let path = folder.join(format!("{name}.json"));
            let old = match std::fs::read(&path) {
                Ok(b) => Some(b),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(e.to_string()),
            };
            files.push((path.clone(), old));
            let key = format!("Software\\{vendor}\\NativeMessagingHosts\\org.idx.downloader");
            for view in [0x200, 0x100] {
                values.push((
                    key.clone(),
                    view,
                    path.to_string_lossy().into_owned(),
                    read(&key, view)?,
                ));
            }
        }
        std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
        let result = (|| {
            for (path, _) in &files {
                std::fs::write(path, contents).map_err(|e| e.to_string())?;
            }
            for (key, view, path, _) in &values {
                write(key, *view, Some(path))?;
            }
            Ok(())
        })();
        if result.is_err() {
            for (key, view, _, old) in &values {
                let _ = write(key, *view, old.as_deref());
            }
            for (path, old) in &files {
                if let Some(old) = old {
                    let _ = std::fs::write(path, old);
                } else {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_are_exact_chromium_origins() {
        assert!(valid_id(&"a".repeat(32)));
        for id in [
            "a".repeat(31),
            "q".repeat(32),
            "A".repeat(32),
            "chrome-extension://abc/".into(),
        ] {
            assert!(!valid_id(&id));
        }
        let ids = vec!["a".repeat(32), "p".repeat(32)];
        let host = std::env::temp_dir().join("App space/idx-native-host.exe");
        let data = manifest(&host, &ids);
        assert_eq!(data["path"], host.to_string_lossy().as_ref());
        assert_eq!(
            data["allowed_origins"],
            serde_json::json!([
                format!("chrome-extension://{}/", ids[0]),
                format!("chrome-extension://{}/", ids[1])
            ])
        );
    }
    #[test]
    fn skip_is_persisted_without_creating_a_browser_registration() {
        let dir = std::env::temp_dir().join(format!(
            "idx-browser-setup-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        assert!(!load(&dir).offered);
        mark_offered(&dir).unwrap();
        assert!(load(&dir).offered);
        assert!(load(&dir).extension_ids.is_empty());
        assert!(!dir.join("native-hosts").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
