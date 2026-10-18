//! Persisted category rules shared by native forms, batch adds and browser handoff.
//! Dialog correspondence: CAddTreeItem (204), COptSave (143). Rule precedence
//! and IDN edge cases still require comparison with the running reference.
use crate::model::{Download, Settings};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Category {
    pub name: String,
    pub extensions: String,
    pub sites_only: bool,
    pub sites: String,
    /// None preserves the pre-category-editor destination policy.
    pub folder: Option<PathBuf>,
    pub remember_last_path: bool,
    /// The legacy Other category remains the fallback even after a rename.
    pub fallback: bool,
}

pub fn defaults() -> Vec<Category> {
    [
        ("Compressed", "zip rar 7z gz bz2 xz tar"),
        ("Documents", "pdf doc docx txt epub xlsx csv pptx"),
        ("Music", "mp3 flac wav aac ogg m4a"),
        ("Programs", "exe msi msix apk iso"),
        ("Video", "mp4 mkv webm mov avi m4v"),
        ("Other", ""),
    ]
    .into_iter()
    .map(|(name, extensions)| Category {
        name: name.into(),
        extensions: extensions.into(),
        fallback: name == "Other",
        ..Default::default()
    })
    .collect()
}

pub fn normalize_extensions(value: &str) -> Result<String, String> {
    let mut extensions = Vec::new();
    for token in value.split_whitespace() {
        let token = token
            .trim_start_matches("*.")
            .trim_start_matches('.')
            .to_lowercase();
        if token.is_empty()
            || token
                .chars()
                .any(|c| c.is_control() || "<>:\"/\\|?*;,.".contains(c))
        {
            return Err("Separate file extensions with spaces, for example: zip pdf mp4.".into());
        }
        if !extensions.contains(&token) {
            extensions.push(token);
        }
    }
    Ok(extensions.join(" "))
}

/// The reference's OK handler (0x406420) strips a supplied scheme and stores
/// host names. URL parsing also canonicalizes case and international domains.
pub fn normalize_sites(value: &str) -> Result<String, String> {
    let mut sites = Vec::new();
    for token in value.split_whitespace() {
        let token = token.split_once("://").map_or(token, |(_, host)| host);
        let url = url::Url::parse(&format!("http://{token}"))
            .map_err(|_| "Enter valid sites separated by spaces; * is allowed as a wildcard.")?;
        if !url.username().is_empty() || url.password().is_some() {
            return Err("Category site rules must not contain login details.".into());
        }
        let host = url.host_str().ok_or("Enter a site name.")?.to_lowercase();
        if !sites.contains(&host) {
            sites.push(host);
        }
    }
    Ok(sites.join(" "))
}

pub fn validate(
    mut category: Category,
    all: &[Category],
    editing: Option<usize>,
) -> Result<Category, String> {
    category.name = category.name.trim().to_string();
    let lower = category.name.to_lowercase();
    if category.name.is_empty()
        || category.name.chars().count() > 128
        || category
            .name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || category.name.ends_with('.')
        || [
            "general",
            "all downloads",
            "unfinished",
            "finished",
            "grabber",
            "queues",
            ".",
            "..",
        ]
        .contains(&lower.as_str())
        || crate::model::safe_filename(&category.name) != category.name
    {
        return Err("Enter a category name of 1 to 128 characters that can also be used as a folder name. This name is reserved or invalid.".into());
    }
    if all
        .iter()
        .enumerate()
        .any(|(i, c)| Some(i) != editing && c.name.to_lowercase() == lower)
    {
        return Err("A category with that name already exists.".into());
    }
    if let Some(folder) = &category.folder {
        if !folder.is_absolute() {
            return Err("Choose an absolute save folder.".into());
        }
    }
    category.extensions = normalize_extensions(&category.extensions)?;
    category.sites = normalize_sites(&category.sites)?;
    // The original handler clears the restriction when no site was entered.
    category.sites_only &= !category.sites.is_empty();
    Ok(category)
}

fn wildcard(pattern: &str, text: &str) -> bool {
    // Linear-space wildcard matching, with '*' as the only wildcard.
    let (p, t) = (pattern.as_bytes(), text.as_bytes());
    let (mut i, mut j, mut star, mut retry) = (0, 0, None, 0);
    while j < t.len() {
        if i < p.len() && p[i] == t[j] {
            i += 1;
            j += 1;
        } else if i < p.len() && p[i] == b'*' {
            star = Some(i);
            i += 1;
            retry = j;
        } else if let Some(s) = star {
            retry += 1;
            j = retry;
            i = s + 1;
        } else {
            return false;
        }
    }
    while i < p.len() && p[i] == b'*' {
        i += 1;
    }
    i == p.len()
}

impl Settings {
    pub fn category_for(&self, filename: &str, source: &str, selected: usize) -> String {
        if let Some(c) = selected.checked_sub(1).and_then(|i| self.categories.get(i)) {
            return c.name.clone();
        }
        let ext = Path::new(filename)
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let host = url::Url::parse(source)
            .ok()
            .and_then(|url| url.host_str().map(str::to_lowercase));
        self.categories
            .iter()
            .find(|c| {
                c.extensions
                    .split_whitespace()
                    .any(|e| e.eq_ignore_ascii_case(&ext))
                    && (!c.sites_only
                        || host.as_ref().is_some_and(|h| {
                            c.sites
                                .split_whitespace()
                                .any(|p| wildcard(&p.to_lowercase(), h))
                        }))
            })
            .or_else(|| self.categories.iter().find(|c| c.fallback))
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Other".into())
    }

    pub fn category_folder(&self, name: &str) -> PathBuf {
        self.categories
            .iter()
            .find(|c| c.name == name)
            .and_then(|c| c.folder.clone())
            .unwrap_or_else(|| {
                if self.category_folders {
                    self.download_dir.join(crate::model::safe_filename(name))
                } else {
                    self.download_dir.clone()
                }
            })
    }

    pub fn destination_folder(&self, name: &str, requested: &Path, exact: bool) -> PathBuf {
        if exact || requested != self.download_dir {
            requested.to_owned()
        } else {
            self.category_folder(name)
        }
    }

    pub fn remember_category_folder(&mut self, name: &str, folder: &Path, explicitly: bool) {
        if let Some(category) = self.categories.iter_mut().find(|c| c.name == name) {
            if explicitly || category.remember_last_path {
                category.folder = Some(folder.to_owned());
            }
        }
        if self.remember_last_path {
            self.download_dir = folder.to_owned();
        }
    }
}

/// Editing is staged in Options. Existing identities are retained by index:
/// this editor supports append and rename, not removal or reordering. Apply all
/// renames from the original names in one pass (including swaps), never paths.
pub fn commit(
    settings: &mut Settings,
    downloads: &mut [Download],
    draft: &Settings,
    filter: &mut String,
) {
    let renames: Vec<_> = settings
        .categories
        .iter()
        .zip(&draft.categories)
        .filter(|(old, new)| old.name != new.name)
        .map(|(old, new)| (old.name.clone(), new.name.clone()))
        .collect();
    for download in downloads {
        if let Some((_, name)) = renames.iter().find(|(old, _)| old == &download.category) {
            download.category = name.clone();
        }
    }
    if let Some((_, name)) = renames.iter().find(|(old, _)| old == filter) {
        *filter = name.clone();
    }
    settings.categories = draft.categories.clone();
    settings.download_dir = draft.download_dir.clone();
    settings.remember_last_path = draft.remember_last_path;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn folder(name: &str) -> PathBuf {
        std::env::temp_dir().join("idx-category-tests").join(name)
    }

    #[test]
    fn old_libraries_keep_rules_and_destination_policy() {
        for subfolders in [false, true] {
            let settings: Settings = serde_json::from_value(
                serde_json::json!({"download_dir":folder("old"),"category_folders":subfolders}),
            )
            .unwrap();
            assert_eq!(settings.categories.len(), 6);
            for (file, expected) in [
                ("DATA.ZIP", "Compressed"),
                ("film.mp4", "Video"),
                ("noext", "Other"),
            ] {
                assert_eq!(
                    settings.category_for(file, "https://example.com/file", 0),
                    expected
                );
                assert_eq!(
                    settings.category_folder(expected),
                    if subfolders {
                        folder("old").join(expected)
                    } else {
                        folder("old")
                    }
                );
            }
        }
    }

    #[test]
    fn rules_require_both_extension_and_whole_host_match() {
        let mut settings = Settings::default();
        let custom = validate(
            Category {
                name: "Books".into(),
                extensions: ".CBZ *.cbr CBZ".into(),
                sites_only: true,
                sites: "https://*.Example.COM/path books.example.org".into(),
                ..Default::default()
            },
            &settings.categories,
            None,
        )
        .unwrap();
        assert_eq!(custom.extensions, "cbz cbr");
        assert_eq!(custom.sites, "*.example.com books.example.org");
        settings.categories.push(custom);
        for url in ["https://a.example.com/x", "https://books.example.org/x"] {
            assert_eq!(settings.category_for("BOOK.CBZ", url, 0), "Books");
        }
        for url in [
            "https://example.com/x",
            "https://a.example.com.evil.test/x",
            "https://example.com@evil.test/x",
        ] {
            assert_eq!(settings.category_for("book.cbz", url, 0), "Other");
        }
        assert_eq!(
            settings.category_for("book.exe", "https://a.example.com", 0),
            "Programs"
        );
        assert_eq!(
            settings.category_for("book.exe", "https://unrelated.test", 7),
            "Books"
        );
    }

    #[test]
    fn international_sites_are_canonicalized_and_empty_restriction_cleared() {
        assert_eq!(
            normalize_sites("https://bücher.example/").unwrap(),
            "xn--bcher-kva.example"
        );
        let c = validate(
            Category {
                name: "New".into(),
                sites_only: true,
                ..Default::default()
            },
            &[],
            None,
        )
        .unwrap();
        assert!(!c.sites_only);
        assert!(normalize_sites("http://user:secret@example.com").is_err());
    }

    #[test]
    fn explicit_destination_wins_and_remember_is_opt_in() {
        let mut s = Settings {
            download_dir: folder("base"),
            category_folders: true,
            ..Default::default()
        };
        s.categories[0].folder = Some(folder("archives"));
        assert_eq!(
            s.destination_folder("Compressed", &s.download_dir, false),
            folder("archives")
        );
        assert_eq!(
            s.destination_folder("Compressed", &s.download_dir, true),
            folder("base")
        );
        assert_eq!(
            s.destination_folder("Compressed", &folder("manual"), false),
            folder("manual")
        );
        s.remember_category_folder("Compressed", &folder("choice"), false);
        assert_eq!(s.category_folder("Compressed"), folder("archives"));
        s.categories[0].remember_last_path = true;
        s.remember_category_folder("Compressed", &folder("choice"), false);
        assert_eq!(s.category_folder("Compressed"), folder("choice"));
        assert_eq!(s.download_dir, folder("base"));
    }

    #[test]
    fn staged_edits_rename_references_once_without_moving_downloads() {
        let mut s = Settings::default();
        let mut draft = s.clone();
        draft.categories[0].name = "Documents".into();
        draft.categories[1].name = "Compressed".into();
        let mut downloads = vec![Download {
            category: "Compressed".into(),
            path: folder("original").join("file.zip"),
            ..Default::default()
        }];
        let path = downloads[0].path.clone();
        let mut filter = "Compressed".into();
        assert_eq!(s.categories[0].name, "Compressed");
        commit(&mut s, &mut downloads, &draft, &mut filter);
        assert_eq!(downloads[0].category, "Documents");
        assert_eq!(downloads[0].path, path);
        assert_eq!(filter, "Documents");
        let restored: Settings = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(restored.categories, s.categories);
    }

    #[test]
    fn validation_rejects_duplicate_reserved_and_unsafe_names_and_relative_folders() {
        let s = Settings::default();
        for name in [
            "compressed",
            "General",
            "queue:test",
            "../escape",
            "NUL",
            "Finished",
            "",
        ] {
            assert!(
                validate(
                    Category {
                        name: name.into(),
                        ..Default::default()
                    },
                    &s.categories,
                    None
                )
                .is_err(),
                "{name}"
            );
        }
        assert!(
            validate(
                Category {
                    name: "Test".into(),
                    folder: Some("relative".into()),
                    ..Default::default()
                },
                &s.categories,
                None
            )
            .is_err()
        );
    }
}
