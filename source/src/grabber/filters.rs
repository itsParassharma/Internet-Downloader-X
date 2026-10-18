//! Grabber filters corresponding to original dialogs 240, 244, 245 and 264.
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct NamedFilter {
    pub name: String,
    pub mask: String,
}
pub fn default_filters() -> Vec<NamedFilter> {
    [
        ("Archives", "*.zip,*.rar,*.7z"),
        ("Media", "*.mp3,*.mp4,*.mkv"),
        ("Documents", "*.pdf,*.doc,*.docx"),
        ("Web pages", "*.html,*.htm"),
        ("Images", "*.jpg,*.png,*.gif,*.svg"),
    ]
    .into_iter()
    .map(|(name, mask)| NamedFilter {
        name: name.into(),
        mask: mask.into(),
    })
    .collect()
}
pub fn validate_named(
    mut draft: NamedFilter,
    filters: &[NamedFilter],
    editing: Option<usize>,
) -> Result<NamedFilter, String> {
    draft.name = draft.name.trim().to_string();
    draft.mask = draft.mask.trim().to_string();
    if draft.name.is_empty()
        || draft.name.chars().count() > 128
        || draft.name.chars().any(char::is_control)
    {
        return Err("Enter a filter name of 1 to 128 characters.".into());
    }
    if ["All files", "None"]
        .iter()
        .any(|name| draft.name.eq_ignore_ascii_case(name))
        || filters
            .iter()
            .enumerate()
            .any(|(i, f)| Some(i) != editing && f.name.to_lowercase() == draft.name.to_lowercase())
    {
        return Err("A filter with this name already exists. Choose a different name.".into());
    }
    if draft.mask.is_empty() {
        return Err("Enter at least one file mask, such as image*.jpg.".into());
    }
    Masks::new(&draft.mask)?;
    Ok(draft)
}

#[derive(Clone, Debug, Default)]
pub struct PathRules {
    pub include: String,
    pub exclude: String,
}
pub struct Paths {
    include: Vec<PathPattern>,
    exclude: Vec<PathPattern>,
}
struct PathPattern {
    host: Option<Regex>,
    scheme: Option<String>,
    port: Option<u16>,
    path: Regex,
}
fn glob(pattern: &str, insensitive: bool, prefix: bool) -> Result<Regex, String> {
    let source = format!(
        "^{}{}$",
        regex::escape(pattern).replace(r"\*", ".*"),
        if prefix && !pattern.contains('*') && !pattern.ends_with('/') {
            "(?:/.*)?"
        } else if prefix && pattern.ends_with('/') {
            ".*"
        } else {
            ""
        }
    );
    RegexBuilder::new(&source)
        .case_insensitive(insensitive)
        .build()
        .map_err(|e| e.to_string())
}
fn entries<'a>(text: &'a str, delimiters: &[char]) -> Result<Vec<&'a str>, String> {
    if text.len() > 8192 || text.chars().any(|c| c.is_control() && !c.is_whitespace()) {
        return Err(
            "Filter patterns are limited to 8,192 characters without control characters.".into(),
        );
    }
    let values = text
        .split(delimiters)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if values.len() > 128 {
        return Err("Use at most 128 patterns in each filter.".into());
    }
    Ok(values)
}
impl PathPattern {
    fn new(pattern: &str) -> Result<Self, String> {
        if pattern == "*" || pattern.starts_with('/') {
            return Ok(Self {
                host: None,
                scheme: None,
                port: None,
                path: glob(pattern, false, true)?,
            });
        }
        let (scheme, rest) = if let Some((scheme, rest)) = pattern.split_once("://") {
            if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") {
                return Err("Path filters support HTTP and HTTPS addresses.".into());
            }
            (Some(scheme.to_ascii_lowercase()), rest)
        } else {
            (None, pattern)
        };
        let (authority, path) = rest
            .split_once('/')
            .map(|(a, p)| (a, format!("/{p}")))
            .unwrap_or((rest, "/".into()));
        if authority.is_empty() || authority.contains(['@', '?', '#']) {
            return Err("Enter a domain or HTTP(S) path without credentials.".into());
        }
        // Let URL parsing normalize IDNs, IPv6 and ports; a placeholder protects '*'.
        let parsed = Url::parse(&format!(
            "{}://{}/",
            scheme.as_deref().unwrap_or("http"),
            authority.replace('*', "idxwildcardtoken")
        ))
        .map_err(|_| "Enter a valid domain or HTTP(S) path.".to_string())?;
        let host = parsed
            .host_str()
            .ok_or("Enter a domain for the path filter.")?
            .replace("idxwildcardtoken", "*");
        let explicit_port = authority
            .rsplit_once(':')
            .filter(|(_, p)| !p.contains(']'))
            .and_then(|(_, p)| p.parse::<u16>().ok());
        Ok(Self {
            host: Some(glob(&host, true, false)?),
            scheme,
            port: explicit_port,
            path: glob(&path, false, true)?,
        })
    }
    fn matches(&self, url: &Url) -> bool {
        self.scheme.as_ref().is_none_or(|s| s == url.scheme())
            && self
                .port
                .is_none_or(|p| Some(p) == url.port_or_known_default())
            && self
                .host
                .as_ref()
                .is_none_or(|h| url.host_str().is_some_and(|s| h.is_match(s)))
            && self.path.is_match(url.path())
    }
}
impl Paths {
    pub fn new(rules: &PathRules) -> Result<Self, String> {
        let parse = |s: &str| -> Result<Vec<PathPattern>, String> {
            entries(s, &[';', '\n', '\r'])?
                .into_iter()
                .map(PathPattern::new)
                .collect()
        };
        Ok(Self {
            include: parse(&rules.include)?,
            exclude: parse(&rules.exclude)?,
        })
    }
    pub fn allows(&self, url: &Url) -> bool {
        (self.include.is_empty() || self.include.iter().any(|p| p.matches(url)))
            && !self.exclude.iter().any(|p| p.matches(url))
    }
}

struct Masks {
    names: Vec<Regex>,
    start_page: bool,
}
impl Masks {
    fn new(text: &str) -> Result<Self, String> {
        let mut names = Vec::new();
        let mut start_page = false;
        for entry in entries(text, &[',', ';', '\n', '\r'])? {
            if entry.eq_ignore_ascii_case("<start page>") {
                start_page = true;
            } else {
                names.push(glob(entry, true, false)?);
            }
        }
        Ok(Self { names, start_page })
    }
    fn matches(&self, url: &Url, base: &Url) -> bool {
        (self.start_page && url == base)
            || self
                .names
                .iter()
                .any(|p| p.is_match(url.path().rsplit('/').next().unwrap_or("")))
    }
    fn empty(&self) -> bool {
        self.names.is_empty() && !self.start_page
    }
}
#[derive(Clone, Debug, Default)]
pub struct FileRules {
    pub include: String,
    pub exclude: String,
    pub site_only: bool,
    pub paths: PathRules,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub hide_duplicates: bool,
}
pub struct Files {
    include: Masks,
    exclude: Masks,
    paths: Paths,
    pub rules: FileRules,
}
impl Files {
    pub fn new(rules: &FileRules) -> Result<Self, String> {
        if rules
            .min_size
            .zip(rules.max_size)
            .is_some_and(|(min, max)| min > max)
        {
            return Err("Minimum file size must not exceed maximum file size.".into());
        }
        Ok(Self {
            include: Masks::new(&rules.include)?,
            exclude: Masks::new(&rules.exclude)?,
            paths: Paths::new(&rules.paths)?,
            rules: rules.clone(),
        })
    }
    pub fn allows(&self, url: &Url, base: &Url) -> bool {
        (!self.rules.site_only || same_site(url, base))
            && self.paths.allows(url)
            && (self.include.empty() || self.include.matches(url, base))
            && !self.exclude.matches(url, base)
    }
    pub fn includes_start_page(&self) -> bool {
        self.include.start_page
    }
    pub fn needs_size(&self) -> bool {
        self.rules.min_size.is_some() || self.rules.max_size.is_some()
    }
    pub fn size_allows(&self, size: Option<u64>) -> bool {
        if !self.needs_size() {
            return true;
        }
        size.is_some_and(|size| {
            self.rules.min_size.is_none_or(|min| size >= min)
                && self.rules.max_size.is_none_or(|max| size <= max)
        })
    }
    /// Redirects keep the original filename mask but must obey site/path restrictions.
    pub fn redirect_allows(&self, url: &Url, base: &Url) -> bool {
        (!self.rules.site_only || same_site(url, base)) && self.paths.allows(url)
    }
}
pub fn same_site(a: &Url, b: &Url) -> bool {
    a.host_str() == b.host_str()
}

/// Original units use powers of 1024 (FUN_0053d8f0); Rust rejects overflow.
pub fn size_bytes(text: &str, unit: usize) -> Result<u64, String> {
    let number = text
        .trim()
        .parse::<u64>()
        .map_err(|_| "Enter a nonnegative whole file size.".to_string())?;
    let multiplier = [1, 1024, 1024 * 1024, 1024 * 1024 * 1024]
        .get(unit)
        .copied()
        .ok_or("Choose Bytes, KB, MB or GB.")?;
    number
        .checked_mul(multiplier)
        .ok_or_else(|| "The file size is too large.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }
    #[test]
    fn filename_masks_match_whole_names_and_start_page_without_query_confusion() {
        let base = url("https://example.com/index.html");
        let f = Files::new(&FileRules {
            include: "image*.jpg,<start page>".into(),
            exclude: "*private*".into(),
            ..Default::default()
        })
        .unwrap();
        assert!(f.allows(&base, &base));
        assert!(f.allows(&url("https://example.com/IMAGE02.JPG?file=no.pdf"), &base));
        for path in [
            "otherimage.jpg",
            "image-private.jpg",
            "image.jpg.exe",
            "file.zip?image.jpg",
        ] {
            assert!(!f.allows(&url(&format!("https://example.com/{path}")), &base));
        }
    }
    #[test]
    fn domain_path_rules_observe_host_and_directory_boundaries_and_exclusions() {
        let p = Paths::new(&PathRules {
            include: "EXAMPLE.com/files;*.cdn.example.com/Media/*;/shared/".into(),
            exclude: "/*/private/*".into(),
        })
        .unwrap();
        assert!(p.allows(&url("https://example.com/files/one.zip")));
        assert!(p.allows(&url("https://a.cdn.example.com/Media/two.zip")));
        assert!(p.allows(&url("https://other.com/shared/a")));
        for s in [
            "https://example.com.evil/files/a",
            "https://example.com/files2/a",
            "https://a.cdn.example.com/media/a",
            "https://example.com/files/private/a",
        ] {
            assert!(!p.allows(&url(s)), "{s}");
        }
    }
    #[test]
    fn path_rules_support_idn_ipv6_explicit_ports_and_protocols() {
        let p = Paths::new(&PathRules {
            include: "https://bücher.example/a;http://[::1]:8080/files".into(),
            ..Default::default()
        })
        .unwrap();
        assert!(p.allows(&url("https://xn--bcher-kva.example/a/b")));
        assert!(p.allows(&url("http://[::1]:8080/files/a")));
        assert!(!p.allows(&url("http://[::1]:8081/files/a")));
        assert!(!p.allows(&url("http://bücher.example/a/b")));
    }
    #[test]
    fn size_units_boundaries_unknowns_and_overflow_are_explicit() {
        assert_eq!(size_bytes(" 3 ", 3).unwrap(), 3 * 1024 * 1024 * 1024);
        assert!(size_bytes("-1", 0).is_err());
        assert!(size_bytes("1.5", 1).is_err());
        assert!(size_bytes(&u64::MAX.to_string(), 1).is_err());
        let f = Files::new(&FileRules {
            min_size: Some(10),
            max_size: Some(20),
            ..Default::default()
        })
        .unwrap();
        for size in [10, 20] {
            assert!(f.size_allows(Some(size)));
        }
        for size in [None, Some(9), Some(21)] {
            assert!(!f.size_allows(size));
        }
        assert!(
            Files::new(&FileRules {
                min_size: Some(20),
                max_size: Some(10),
                ..Default::default()
            })
            .is_err()
        );
    }
    #[test]
    fn named_filters_validate_case_insensitive_uniqueness_and_persist_in_old_settings() {
        let mut settings: crate::model::Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.grabber_filters, default_filters());
        let f = validate_named(
            NamedFilter {
                name: " Pictures ".into(),
                mask: "image*.jpg".into(),
            },
            &settings.grabber_filters,
            None,
        )
        .unwrap();
        settings.grabber_filters.push(f.clone());
        assert!(
            validate_named(
                NamedFilter {
                    name: "pictures".into(),
                    ..f.clone()
                },
                &settings.grabber_filters,
                None
            )
            .is_err()
        );
        assert!(
            validate_named(
                f.clone(),
                &settings.grabber_filters,
                Some(settings.grabber_filters.len() - 1)
            )
            .is_ok()
        );
        let restored: crate::model::Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored.grabber_filters.last(), Some(&f));
        settings.grabber_filters.clear();
        let restored: crate::model::Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert!(restored.grabber_filters.is_empty());
    }
    #[test]
    fn site_identity_matches_reference_hostname_across_ports_and_protocols() {
        assert!(same_site(
            &url("https://example.com:8443/a"),
            &url("http://example.com/b")
        ));
        assert!(!same_site(
            &url("https://other.example.com/a"),
            &url("https://example.com/b")
        ));
    }
}
