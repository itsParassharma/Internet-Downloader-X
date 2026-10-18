use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    future::{Future, poll_fn},
    sync::mpsc::Sender,
    task::Poll,
    time::Duration,
};
use url::Url;
pub mod filters;
use filters::{FileRules, Files, PathRules, Paths, same_site};

pub const MAX_REQUESTS: usize = 250;
const MAX_LINKS: usize = 20_000;
const MAX_PAGE_BYTES: usize = 4 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct ScanOptions {
    /// Zero explores only the start page (original grabber.chm/nlevels.html).
    pub levels: usize,
    pub other_levels: usize,
    pub no_parent_directories: bool,
    pub max_requests: usize,
    pub page_paths: PathRules,
    pub files: FileRules,
}
impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            levels: 0,
            other_levels: 0,
            no_parent_directories: false,
            max_requests: MAX_REQUESTS,
            page_paths: PathRules::default(),
            files: FileRules::default(),
        }
    }
}

#[derive(Debug)]
pub struct PageLinks {
    pub page_url: String,
    pub urls: Vec<String>,
    /// Final URL of the first page that actually contained each link.
    pub parents: BTreeMap<String, String>,
    pub pages: usize,
    pub skipped: usize,
    pub limited: bool,
    pub unknown_sizes: usize,
    pub duplicates: usize,
}
impl PageLinks {
    pub fn summary(&self) -> String {
        format!(
            "{} pages, {} links{}{}{}{}",
            self.pages,
            self.urls.len(),
            if self.limited {
                " (scan limit reached)"
            } else {
                ""
            },
            if self.skipped > 0 {
                format!(", {} pages skipped", self.skipped)
            } else {
                String::new()
            },
            if self.unknown_sizes > 0 {
                format!(", {} sizes unknown", self.unknown_sizes)
            } else {
                String::new()
            },
            if self.duplicates > 0 {
                format!(", {} duplicates hidden", self.duplicates)
            } else {
                String::new()
            }
        )
    }
}
pub enum ScanEvent {
    Progress { pages: usize, links: usize },
    Checking { checked: usize, total: usize },
    Finished(Result<PageLinks, String>),
}

/// Dropping the handle cancels active HTTP requests and queued work.
pub struct ScanHandle(Option<tokio::sync::oneshot::Sender<()>>);
impl Drop for ScanHandle {
    fn drop(&mut self) {
        if let Some(cancel) = self.0.take() {
            let _ = cancel.send(());
        }
    }
}

pub fn scan(page: String, options: ScanOptions, tx: Sender<ScanEvent>) -> ScanHandle {
    let (cancel, cancelled) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let result = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime.block_on(async {
                let mut work = std::pin::pin!(crawl(page, options, &tx));
                let mut cancelled = std::pin::pin!(cancelled);
                poll_fn(|cx| {
                    if cancelled.as_mut().poll(cx).is_ready() {
                        Poll::Ready(Err("Scan cancelled.".into()))
                    } else {
                        work.as_mut().poll(cx)
                    }
                })
                .await
            }),
            Err(error) => Err(error.to_string()),
        };
        let _ = tx.send(ScanEvent::Finished(result));
    });
    ScanHandle(Some(cancel))
}

fn normalized(mut url: Url) -> Option<Url> {
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    url.set_fragment(None);
    Some(url)
}

#[derive(Clone)]
struct Page {
    url: Url,
    level: usize,
    other_level: usize,
}
fn next_page(url: Url, parent: &Page, base: &Url, options: &ScanOptions) -> Option<Page> {
    let (level, other_level) = if same_site(&url, base) {
        (parent.level + 1, 0)
    } else {
        (parent.level, parent.other_level + 1)
    };
    let page = Page {
        url,
        level,
        other_level,
    };
    allowed(&page, base, options).then_some(page)
}
fn allowed(page: &Page, base: &Url, options: &ScanOptions) -> bool {
    if same_site(&page.url, base) {
        let directory = base
            .path()
            .rsplit_once('/')
            .map(|(d, _)| format!("{d}/"))
            .unwrap_or("/".into());
        page.level <= options.levels
            && (!options.no_parent_directories || page.url.path().starts_with(&directory))
    } else {
        page.other_level > 0 && page.other_level <= options.other_levels
    }
}

// Follow navigation links; file/media references remain results.
// Non-HTML responses are never consumed as page bodies.
fn could_be_page(url: &Url) -> bool {
    let last = url.path().rsplit('/').next().unwrap_or_default();
    let extension = last.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    extension.is_none()
        || matches!(
            extension.as_deref(),
            Some("html" | "htm" | "shtml" | "xhtml" | "php" | "asp" | "aspx" | "jsp" | "cfm")
        )
}

struct HtmlLinks {
    tags: regex::Regex,
    attributes: regex::Regex,
    ignored: regex::Regex,
}
impl HtmlLinks {
    fn new() -> Self {
        Self {
            tags: regex::Regex::new(r#"(?is)<([a-z][a-z0-9:-]*)\b((?:[^>"']|"[^"]*"|'[^']*')*)>"#)
                .unwrap(),
            attributes: regex::Regex::new(
                r#"(?is)([a-z_:][a-z0-9_:.-]*)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#,
            )
            .unwrap(),
            ignored: regex::Regex::new(
                r"(?is)<!--.*?-->|<script\b[^>]*>.*?</script\s*>|<style\b[^>]*>.*?</style\s*>",
            )
            .unwrap(),
        }
    }
    fn extract(&self, page: &Url, text: &str) -> Vec<(Url, bool)> {
        let text = self.ignored.replace_all(text, "");
        let tags = self.tags.captures_iter(&text).collect::<Vec<_>>();
        let mut base = page.clone();
        for tag in &tags {
            if tag[1].eq_ignore_ascii_case("base") {
                if let Some(value) = self
                    .attributes
                    .captures_iter(&tag[2])
                    .find(|a| a[1].eq_ignore_ascii_case("href"))
                {
                    let value = value
                        .get(2)
                        .or_else(|| value.get(3))
                        .or_else(|| value.get(4))
                        .unwrap()
                        .as_str();
                    if let Some(url) = page.join(&html_entities(value)).ok().and_then(normalized) {
                        base = url;
                    }
                    break;
                }
            }
        }
        let mut links = Vec::new();
        for tag in tags {
            if tag[1].eq_ignore_ascii_case("base") {
                continue;
            }
            for attribute in self.attributes.captures_iter(&tag[2]) {
                let name = &attribute[1];
                if !name.eq_ignore_ascii_case("href") && !name.eq_ignore_ascii_case("src") {
                    continue;
                }
                let value = attribute
                    .get(2)
                    .or_else(|| attribute.get(3))
                    .or_else(|| attribute.get(4))
                    .unwrap()
                    .as_str()
                    .trim();
                if value.is_empty() || value.starts_with('#') {
                    continue;
                }
                if let Some(url) = base.join(&html_entities(value)).ok().and_then(normalized) {
                    let navigation = (name.eq_ignore_ascii_case("href")
                        && (tag[1].eq_ignore_ascii_case("a")
                            || tag[1].eq_ignore_ascii_case("area")))
                        || (name.eq_ignore_ascii_case("src")
                            && (tag[1].eq_ignore_ascii_case("frame")
                                || tag[1].eq_ignore_ascii_case("iframe")));
                    links.push((url, navigation));
                }
            }
        }
        links
    }
}
fn html_entities(text: &str) -> String {
    let mut output = String::new();
    let mut remaining = text;
    while let Some(index) = remaining.find('&') {
        output.push_str(&remaining[..index]);
        remaining = &remaining[index + 1..];
        if let Some(end) = remaining.find(';').filter(|n| *n < 12) {
            let entity = &remaining[..end];
            let decoded = match entity {
                "amp" => Some('&'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "lt" => Some('<'),
                "gt" => Some('>'),
                _ => entity
                    .strip_prefix("#x")
                    .or_else(|| entity.strip_prefix("#X"))
                    .and_then(|n| u32::from_str_radix(n, 16).ok())
                    .or_else(|| entity.strip_prefix('#').and_then(|n| n.parse().ok()))
                    .and_then(char::from_u32),
            };
            if let Some(value) = decoded {
                output.push(value);
                remaining = &remaining[end + 1..];
                continue;
            }
        }
        output.push('&');
    }
    output.push_str(remaining);
    output
}

async fn crawl(
    page: String,
    options: ScanOptions,
    tx: &Sender<ScanEvent>,
) -> Result<PageLinks, String> {
    let page_paths = Paths::new(&options.page_paths)?;
    let files = Files::new(&options.files)?;
    let mut base = Url::parse(&page)
        .ok()
        .and_then(normalized)
        .ok_or("Use an HTTP or HTTPS page without embedded credentials.")?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let parser = HtmlLinks::new();
    let mut queue = VecDeque::from([Page {
        url: base.clone(),
        level: 0,
        other_level: 0,
    }]);
    let mut scheduled = HashSet::from([base.to_string()]);
    let mut visited = HashSet::new();
    let mut links = BTreeMap::new();
    let mut discovery_order = Vec::new();
    let (mut requests, mut pages, mut skipped, mut bytes) = (0, 0, 0, 0);
    let mut limited = false;
    let request_limit = options.max_requests.clamp(1, MAX_REQUESTS);
    'pages: while let Some(mut current) = queue.pop_front() {
        if visited.contains(current.url.as_str()) {
            continue;
        }
        let initial = requests == 0;
        let mut redirects = HashSet::new();
        let mut response = loop {
            if requests >= request_limit {
                limited = true;
                break 'pages;
            }
            if !redirects.insert(current.url.to_string()) || redirects.len() > 10 {
                if initial {
                    return Err(
                        "The starting page has a redirect loop or too many redirects.".into(),
                    );
                }
                skipped += 1;
                continue 'pages;
            }
            visited.insert(current.url.to_string());
            requests += 1;
            let response = match client
                .get(current.url.clone())
                .send()
                .await
                .and_then(|r| r.error_for_status())
            {
                Ok(response) => response,
                Err(e) => {
                    if initial {
                        return Err(e.to_string());
                    }
                    skipped += 1;
                    continue 'pages;
                }
            };
            if response.status().is_redirection() {
                let target = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|h| h.to_str().ok())
                    .and_then(|location| current.url.join(location).ok())
                    .and_then(normalized);
                let Some(target) = target else {
                    if initial {
                        return Err("The starting page returned an invalid redirect.".into());
                    }
                    skipped += 1;
                    continue 'pages;
                };
                if !initial {
                    let other_level = if same_site(&target, &base) {
                        0
                    } else {
                        current.other_level.max(1)
                    };
                    let redirected = Page {
                        url: target.clone(),
                        other_level,
                        level: current.level
                            + usize::from(
                                same_site(&target, &base) && !same_site(&current.url, &base),
                            ),
                        ..current.clone()
                    };
                    if !allowed(&redirected, &base, &options)
                        || !page_paths.allows(&target)
                        || visited.contains(target.as_str())
                    {
                        skipped += 1;
                        continue 'pages;
                    }
                    current = redirected;
                } else {
                    current.url = target;
                }
                continue;
            }
            break response;
        };
        if initial {
            base = current.url.clone();
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim();
        if !content_type.is_empty()
            && !content_type.eq_ignore_ascii_case("text/html")
            && !content_type.eq_ignore_ascii_case("application/xhtml+xml")
        {
            if initial {
                return Err("The starting address did not return an HTML page.".into());
            }
            continue;
        }
        let mut body = Vec::new();
        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    bytes += chunk.len();
                    if bytes > MAX_TOTAL_BYTES {
                        limited = true;
                        break 'pages;
                    }
                    if body.len() + chunk.len() > MAX_PAGE_BYTES {
                        if initial {
                            return Err(
                                "The starting HTML page exceeds the 4 MiB scan limit.".into()
                            );
                        }
                        skipped += 1;
                        limited = true;
                        continue 'pages;
                    }
                    body.extend_from_slice(&chunk);
                }
                Ok(None) => break,
                Err(e) => {
                    if initial {
                        return Err(e.to_string());
                    }
                    skipped += 1;
                    continue 'pages;
                }
            }
        }
        pages += 1;
        if initial && files.includes_start_page() {
            discovery_order.push(base.to_string());
            links.insert(base.to_string(), base.to_string());
        }
        for (url, navigation) in parser.extract(&current.url, &String::from_utf8_lossy(&body)) {
            if links.len() >= MAX_LINKS && !links.contains_key(url.as_str()) {
                limited = true;
                break 'pages;
            }
            if !links.contains_key(url.as_str()) {
                discovery_order.push(url.to_string());
                links.insert(url.to_string(), current.url.to_string());
            }
            if navigation && could_be_page(&url) && page_paths.allows(&url) {
                if let Some(next) = next_page(url, &current, &base, &options) {
                    if scheduled.insert(next.url.to_string()) {
                        queue.push_back(next);
                    }
                }
            }
        }
        if tx
            .send(ScanEvent::Progress {
                pages,
                links: links.len(),
            })
            .is_err()
        {
            return Err("Scan cancelled.".into());
        }
    }
    let mut accepted = Vec::new();
    let mut seen_files = HashSet::new();
    let (mut metadata_requests, mut unknown_sizes, mut duplicates) = (0, 0, 0);
    let candidates = discovery_order
        .into_iter()
        .filter_map(|s| {
            let url = Url::parse(&s).ok()?;
            files.allows(&url, &base).then_some(url)
        })
        .collect::<Vec<_>>();
    for (index, url) in candidates.iter().enumerate() {
        let size = if files.needs_size() || files.rules.hide_duplicates {
            if tx
                .send(ScanEvent::Checking {
                    checked: index,
                    total: candidates.len(),
                })
                .is_err()
            {
                return Err("Scan cancelled.".into());
            }
            let result = file_size(
                &client,
                url,
                &base,
                &files,
                &mut metadata_requests,
                request_limit,
                &mut limited,
            )
            .await;
            if !result.allowed {
                continue;
            }
            if result.size.is_none() {
                unknown_sizes += 1;
            }
            result.size
        } else {
            None
        };
        if !files.size_allows(size) {
            continue;
        }
        if files.rules.hide_duplicates {
            if let Some(size) = size {
                let name = crate::model::filename_from_url(url.as_str())
                    .unwrap_or_default()
                    .to_lowercase();
                if !seen_files.insert((name, size)) {
                    duplicates += 1;
                    continue;
                }
            }
        }
        accepted.push(url.to_string());
    }
    let accepted_set = accepted.iter().collect::<HashSet<_>>();
    links.retain(|url, _| accepted_set.contains(url));
    Ok(PageLinks {
        page_url: base.to_string(),
        urls: accepted,
        parents: links,
        pages,
        skipped,
        limited,
        unknown_sizes,
        duplicates,
    })
}

struct FileSize {
    allowed: bool,
    size: Option<u64>,
}
/// HEAD only: neither unknown lengths nor duplicate checking trigger file downloads.
async fn file_size(
    client: &reqwest::Client,
    url: &Url,
    base: &Url,
    files: &Files,
    requests: &mut usize,
    limit: usize,
    limited: &mut bool,
) -> FileSize {
    let unknown = || FileSize {
        allowed: true,
        size: None,
    };
    let mut current = url.clone();
    let mut visited = HashSet::new();
    loop {
        if !files.redirect_allows(&current, base) {
            return FileSize {
                allowed: false,
                size: None,
            };
        }
        if !visited.insert(current.to_string()) || visited.len() > 10 {
            return unknown();
        }
        if *requests >= limit {
            *limited = true;
            return unknown();
        }
        *requests += 1;
        let Ok(response) = client
            .head(current.clone())
            .send()
            .await
            .and_then(|r| r.error_for_status())
        else {
            return unknown();
        };
        if response.status().is_redirection() {
            let Some(target) = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|h| h.to_str().ok())
                .and_then(|location| current.join(location).ok())
                .and_then(normalized)
            else {
                return unknown();
            };
            current = target;
        } else {
            return FileSize {
                allowed: true,
                size: response
                    .headers()
                    .get(reqwest::header::CONTENT_LENGTH)
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse().ok()),
            };
        }
    }
}

#[cfg(test)]
mod tests;
