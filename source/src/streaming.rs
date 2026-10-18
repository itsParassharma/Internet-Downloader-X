//! Bounded HLS VOD / static DASH planning. Network I/O and assembly live elsewhere.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
const MAX_SEGMENTS: usize = 100_000;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct StreamOptions {
    pub kind: String,
    pub max_height: u32,
}
impl StreamOptions {
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.kind.as_str(), "" | "hls" | "dash") || self.max_height > 16384 {
            Err("Invalid stream selection.".into())
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Encryption {
    pub url: String,
    pub iv: [u8; 16],
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Segment {
    pub url: String,
    pub range: Option<(u64, u64)>,
    pub encryption: Option<Encryption>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Track {
    pub kind: String,
    pub segments: Vec<Segment>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Plan {
    pub tracks: Vec<Track>,
}
#[derive(Clone, Debug, Serialize)]
#[allow(dead_code)]
pub struct Quality {
    pub height: u32,
    pub bandwidth: u64,
    pub label: String,
}
pub type Fetch<'a> = dyn FnMut(&str) -> Result<(String, String), String> + 'a;
#[cfg(test)]
#[path = "streaming_tests.rs"]
mod tests;
fn joined(base: &str, reference: &str) -> Result<String, String> {
    let u = Url::parse(base)
        .and_then(|u| u.join(reference))
        .map_err(|_| "Invalid media URL.")?;
    crate::browser_session::network_url(u.as_str()).map(|u| u.to_string())
}
fn attrs(raw: &str) -> Result<HashMap<String, String>, String> {
    let mut out = HashMap::new();
    let mut rest = raw.trim();
    while !rest.is_empty() {
        let (key, tail) = rest.split_once('=').ok_or("Invalid playlist attributes.")?;
        let (value, tail) = if let Some(tail) = tail.strip_prefix('"') {
            let (value, tail) = tail.split_once('"').ok_or("Unclosed playlist attribute.")?;
            (value, tail)
        } else {
            tail.split_once(',').unwrap_or((tail, ""))
        };
        out.insert(key.trim().into(), value.into());
        rest = tail.trim_start_matches(',').trim();
    }
    Ok(out)
}
#[derive(Clone)]
struct Variant {
    url: String,
    height: u32,
    bandwidth: u64,
    audio: String,
}
fn hls_variants(
    text: &str,
    base: &str,
) -> Result<(Vec<Variant>, Vec<HashMap<String, String>>), String> {
    if !text
        .trim_start_matches('\u{feff}')
        .trim_start()
        .starts_with("#EXTM3U")
    {
        return Err("The server did not return an HLS playlist.".into());
    }
    let lines: Vec<_> = text.lines().map(str::trim).collect();
    let mut variants = vec![];
    let mut audio = vec![];
    for (i, line) in lines.iter().enumerate() {
        if let Some(raw) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            let a = attrs(raw)?;
            let next = lines[i + 1..]
                .iter()
                .find(|s| !s.is_empty() && !s.starts_with('#'))
                .ok_or("Missing HLS variant URL.")?;
            variants.push(Variant {
                url: joined(base, next)?,
                height: a
                    .get("RESOLUTION")
                    .and_then(|s| s.split_once('x'))
                    .and_then(|(_, h)| h.parse().ok())
                    .unwrap_or(0),
                bandwidth: a.get("BANDWIDTH").and_then(|x| x.parse().ok()).unwrap_or(0),
                audio: a.get("AUDIO").cloned().unwrap_or_default(),
            });
        } else if let Some(raw) = line.strip_prefix("#EXT-X-MEDIA:") {
            let a = attrs(raw)?;
            if a.get("TYPE").is_some_and(|t| t == "AUDIO") {
                audio.push(a);
            }
        }
    }
    Ok((variants, audio))
}
fn select<'a, T>(
    items: &'a [T],
    limit: u32,
    height: impl Fn(&T) -> u32,
    bandwidth: impl Fn(&T) -> u64,
) -> Option<&'a T> {
    items
        .iter()
        .filter(|v| limit == 0 || height(v) <= limit)
        .max_by_key(|v| (height(v), bandwidth(v)))
        .or_else(|| items.iter().min_by_key(|v| (height(v), bandwidth(v))))
}
fn byte_range(raw: &str, previous: Option<(String, u64)>, url: &str) -> Result<(u64, u64), String> {
    let (len, at) = raw
        .split_once('@')
        .map(|(n, o)| (n, Some(o)))
        .unwrap_or((raw, None));
    let len: u64 = len.parse().map_err(|_| "Invalid HLS range length.")?;
    let start = match at {
        Some(v) => v.parse().map_err(|_| "Invalid HLS range offset.")?,
        None => previous
            .filter(|(u, _)| u == url)
            .map(|(_, end)| end)
            .ok_or("HLS range has no preceding offset for this resource.")?,
    };
    let end = start
        .checked_add(len)
        .filter(|_| len > 0)
        .ok_or("HLS byte range overflow.")?;
    Ok((start, end - 1))
}
fn hls_track(text: &str, base: &str, kind: &str) -> Result<Track, String> {
    if !text.lines().any(|s| s.trim() == "#EXT-X-ENDLIST") {
        return Err(
            "This HLS broadcast is still live. Recording live streams is not supported yet.".into(),
        );
    }
    let mut segments = vec![];
    let mut key: Option<(String, Option<[u8; 16]>)> = None;
    let mut sequence = 0u128;
    let mut range = None;
    let mut previous = None;
    for line in text.lines().map(str::trim) {
        if let Some(raw) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
            sequence = raw.parse().map_err(|_| "Invalid HLS sequence.")?;
        } else if let Some(raw) = line.strip_prefix("#EXT-X-KEY:") {
            let a = attrs(raw)?;
            match a.get("METHOD").map(String::as_str) {
                Some("NONE") => key = None,
                Some("AES-128") if a.get("KEYFORMAT").is_none_or(|v| v == "identity") => {
                    let iv = a
                        .get("IV")
                        .map(|v| {
                            let value = v.strip_prefix("0x").unwrap_or(v);
                            u128::from_str_radix(value, 16)
                                .map(u128::to_be_bytes)
                                .map_err(|_| "Invalid HLS encryption IV.")
                        })
                        .transpose()?;
                    key = Some((
                        joined(base, a.get("URI").ok_or("Missing HLS key address.")?)?,
                        iv,
                    ));
                }
                _ => return Err("This stream uses DRM or unsupported sample encryption.".into()),
            }
        } else if let Some(raw) = line.strip_prefix("#EXT-X-MAP:") {
            let a = attrs(raw)?;
            let url = joined(base, a.get("URI").ok_or("Missing HLS initialization URL.")?)?;
            let range = a
                .get("BYTERANGE")
                .map(|r| byte_range(r, None, &url))
                .transpose()?;
            let encryption = key
                .as_ref()
                .map(|(u, iv)| {
                    iv.map(|iv| Encryption { url: u.clone(), iv })
                        .ok_or("Encrypted HLS initialization requires an explicit IV.")
                })
                .transpose()?;
            segments.push(Segment {
                url,
                range,
                encryption,
            });
        } else if let Some(raw) = line.strip_prefix("#EXT-X-BYTERANGE:") {
            range = Some(raw.to_string());
        } else if line == "#EXT-X-DISCONTINUITY" {
            return Err("HLS discontinuities are not supported in this release.".into());
        } else if !line.is_empty() && !line.starts_with('#') {
            let url = joined(base, line)?;
            let bytes = range
                .take()
                .map(|r| byte_range(&r, previous.take(), &url))
                .transpose()?;
            previous = bytes.map(|(_, end)| (url.clone(), end + 1));
            segments.push(Segment {
                url,
                range: bytes,
                encryption: key.as_ref().map(|(url, iv)| Encryption {
                    url: url.clone(),
                    iv: iv.unwrap_or(sequence.to_be_bytes()),
                }),
            });
            sequence = sequence.checked_add(1).ok_or("HLS sequence overflow.")?;
        }
        if segments.len() > MAX_SEGMENTS {
            return Err("The playlist contains too many segments.".into());
        }
    }
    if segments.is_empty() {
        return Err("The HLS playlist contains no media segments.".into());
    }
    Ok(Track {
        kind: kind.into(),
        segments,
    })
}
pub fn plan(url: &str, options: &StreamOptions, fetch: &mut Fetch<'_>) -> Result<Plan, String> {
    options.validate()?;
    let (base, text) = fetch(url)?;
    if options.kind == "dash" || text.trim_start().starts_with('<') {
        return dash(&text, &base, options.max_height);
    }
    let (variants, audio) = hls_variants(&text, &base)?;
    if variants.is_empty() {
        return Ok(Plan {
            tracks: vec![hls_track(&text, &base, "video")?],
        });
    }
    let variant = select(&variants, options.max_height, |v| v.height, |v| v.bandwidth)
        .ok_or("No HLS variants.")?;
    let (video_base, video) = fetch(&variant.url)?;
    if !hls_variants(&video, &video_base)?.0.is_empty() {
        return Err("Nested HLS master playlists are unsupported.".into());
    }
    let mut tracks = vec![hls_track(&video, &video_base, "video")?];
    if !variant.audio.is_empty() {
        let matching: Vec<_> = audio
            .iter()
            .filter(|a| a.get("GROUP-ID") == Some(&variant.audio))
            .collect();
        let a = matching
            .iter()
            .find(|a| a.get("DEFAULT").is_some_and(|x| x == "YES"))
            .or(matching.first())
            .ok_or("Missing HLS audio group.")?;
        if let Some(uri) = a.get("URI") {
            let (audio_base, text) = fetch(&joined(&base, uri)?)?;
            tracks.push(hls_track(&text, &audio_base, "audio")?);
        }
    }
    Ok(Plan { tracks })
}
fn child<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    n.children().find(|n| n.has_tag_name(name))
}
fn inherited<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<&'a str> {
    n.ancestors().find_map(|n| n.attribute(name))
}
fn base_url(n: roxmltree::Node<'_, '_>, url: &str) -> Result<String, String> {
    let mut chain: Vec<_> = n.ancestors().filter(|n| n.is_element()).collect();
    chain.reverse();
    let mut base = url.to_string();
    for n in chain {
        if let Some(text) = child(n, "BaseURL").and_then(|n| n.text()) {
            base = joined(&base, text.trim())?;
        }
    }
    Ok(base)
}
fn duration(raw: &str) -> Result<f64, String> {
    let raw = raw.strip_prefix("PT").ok_or("Unsupported DASH duration.")?;
    let mut num = String::new();
    let mut total = 0.0;
    for c in raw.chars() {
        if c.is_ascii_digit() || c == '.' {
            num.push(c);
        } else {
            let n: f64 = num.parse().map_err(|_| "Invalid DASH duration.")?;
            num.clear();
            total += n * match c {
                'H' => 3600.0,
                'M' => 60.0,
                'S' => 1.0,
                _ => return Err("Unsupported DASH duration.".into()),
            };
        }
    }
    if !num.is_empty() || !total.is_finite() || total <= 0.0 || total > 604800.0 {
        return Err("Invalid or excessive DASH duration.".into());
    }
    Ok(total)
}
fn expanded(raw: &str, id: &str, bw: u64, number: u64, time: u64) -> Result<String, String> {
    let mut output = String::new();
    let mut rest = raw;
    while let Some(at) = rest.find('$') {
        output.push_str(&rest[..at]);
        rest = &rest[at + 1..];
        if let Some(t) = rest.strip_prefix('$') {
            output.push('$');
            rest = t;
            continue;
        }
        let (token, tail) = rest
            .split_once('$')
            .ok_or("Unclosed DASH template token.")?;
        rest = tail;
        let (token, width) = token
            .split_once("%0")
            .map(|(t, w)| (t, Some(w)))
            .unwrap_or((token, None));
        if token == "RepresentationID" {
            if width.is_some() {
                return Err("Invalid representation template.".into());
            }
            output.push_str(id);
            continue;
        }
        let value = match token {
            "Bandwidth" => bw,
            "Number" => number,
            "Time" => time,
            _ => return Err("Unsupported DASH template token.".into()),
        };
        let width = width
            .map(|w| {
                w.strip_suffix('d')
                    .and_then(|s| s.parse::<usize>().ok())
                    .filter(|n| *n <= 20)
                    .ok_or("Invalid DASH template width.")
            })
            .transpose()?
            .unwrap_or(0);
        output.push_str(&format!("{value:0width$}"));
    }
    output.push_str(rest);
    Ok(output)
}
fn range(raw: &str) -> Result<(u64, u64), String> {
    let (s, e) = raw.split_once('-').ok_or("Invalid DASH byte range.")?;
    let s = s.parse().map_err(|_| "Invalid range.")?;
    let e = e.parse().map_err(|_| "Invalid range.")?;
    if e < s {
        return Err("Inverted DASH byte range.".into());
    }
    Ok((s, e))
}
fn representation(n: roxmltree::Node<'_, '_>, url: &str, seconds: f64) -> Result<Track, String> {
    let base = base_url(n, url)?;
    let id = n.attribute("id").unwrap_or("");
    let bw = n
        .attribute("bandwidth")
        .unwrap_or("0")
        .parse()
        .map_err(|_| "Invalid DASH bandwidth.")?;
    let kind = if inherited(n, "contentType") == Some("audio")
        || inherited(n, "mimeType").is_some_and(|m| m.starts_with("audio/"))
    {
        "audio"
    } else {
        "video"
    };
    let mut segments = vec![];
    let hierarchy: Vec<_> = n.ancestors().filter(|n| n.is_element()).collect();
    let template: Vec<_> = hierarchy
        .iter()
        .filter_map(|n| child(*n, "SegmentTemplate"))
        .collect();
    let attr = |name| template.iter().find_map(|t| t.attribute(name));
    if !template.is_empty() {
        let media = attr("media").ok_or("DASH template has no media address.")?;
        let scale: u64 = attr("timescale")
            .unwrap_or("1")
            .parse()
            .map_err(|_| "Invalid DASH timescale.")?;
        if scale == 0 {
            return Err("Zero DASH timescale.".into());
        }
        let offset: u64 = attr("presentationTimeOffset")
            .unwrap_or("0")
            .parse()
            .map_err(|_| "Invalid DASH time offset.")?;
        let start: u64 = attr("startNumber")
            .unwrap_or("1")
            .parse()
            .map_err(|_| "Invalid DASH start number.")?;
        if let Some(init) = attr("initialization") {
            segments.push(Segment {
                url: joined(&base, &expanded(init, id, bw, start, 0)?)?,
                range: None,
                encryption: None,
            });
        }
        let mut times = vec![];
        if let Some(timeline) = template.iter().find_map(|t| child(*t, "SegmentTimeline")) {
            let entries: Vec<_> = timeline
                .children()
                .filter(|n| n.has_tag_name("S"))
                .collect();
            let mut current = 0u64;
            for (index, s) in entries.iter().enumerate() {
                if let Some(t) = s.attribute("t") {
                    current = t.parse().map_err(|_| "Invalid DASH timeline time.")?;
                }
                let d: u64 = s
                    .attribute("d")
                    .ok_or("Missing DASH segment duration.")?
                    .parse()
                    .map_err(|_| "Invalid DASH segment duration.")?;
                if d == 0 {
                    return Err("Zero DASH segment duration.".into());
                }
                let r: i64 = s
                    .attribute("r")
                    .unwrap_or("0")
                    .parse()
                    .map_err(|_| "Invalid DASH repeat count.")?;
                let count = if r == -1 {
                    let end = entries
                        .get(index + 1)
                        .and_then(|n| n.attribute("t"))
                        .map(|t| t.parse::<u64>().map_err(|_| "Invalid DASH next time."))
                        .transpose()?
                        .unwrap_or(
                            ((seconds * scale as f64).ceil() as u64)
                                .checked_add(offset)
                                .ok_or("DASH time overflow.")?,
                        );
                    end.checked_sub(current)
                        .ok_or("Invalid DASH timeline order.")?
                        .div_ceil(d)
                } else if r >= 0 {
                    (r as u64) + 1
                } else {
                    return Err("Invalid DASH negative repeat.".into());
                };
                if count > MAX_SEGMENTS as u64 || times.len() + count as usize > MAX_SEGMENTS {
                    return Err("Too many DASH segments.".into());
                }
                for _ in 0..count {
                    times.push(current);
                    current = current.checked_add(d).ok_or("DASH timeline overflow.")?;
                }
            }
        } else {
            let d: u64 = attr("duration")
                .ok_or("Missing DASH segment duration.")?
                .parse()
                .map_err(|_| "Invalid DASH duration.")?;
            if d == 0 {
                return Err("Zero DASH segment duration.".into());
            }
            let count = (seconds * scale as f64 / d as f64).ceil() as usize;
            if count == 0 || count > MAX_SEGMENTS {
                return Err("Invalid DASH segment count.".into());
            }
            for i in 0..count {
                times.push((i as u64).checked_mul(d).ok_or("DASH time overflow.")?);
            }
        }
        for (i, t) in times.into_iter().enumerate() {
            segments.push(Segment {
                url: joined(
                    &base,
                    &expanded(
                        media,
                        id,
                        bw,
                        start.checked_add(i as u64).ok_or("DASH number overflow.")?,
                        t,
                    )?,
                )?,
                range: None,
                encryption: None,
            });
        }
    } else if let Some(list) = hierarchy.iter().find_map(|n| child(*n, "SegmentList")) {
        if let Some(init) = child(list, "Initialization") {
            segments.push(Segment {
                url: joined(&base, init.attribute("sourceURL").unwrap_or(""))?,
                range: init.attribute("range").map(range).transpose()?,
                encryption: None,
            });
        }
        for s in list.children().filter(|n| n.has_tag_name("SegmentURL")) {
            segments.push(Segment {
                url: joined(&base, s.attribute("media").unwrap_or(""))?,
                range: s.attribute("mediaRange").map(range).transpose()?,
                encryption: None,
            });
        }
    } else {
        // SegmentBase describes ranges within one complete ISO BMFF resource.
        if base == url {
            return Err("DASH representation has no media addressing.".into());
        }
        segments.push(Segment {
            url: base,
            range: None,
            encryption: None,
        });
    }
    if segments.is_empty() || segments.len() > MAX_SEGMENTS + 1 {
        return Err("Invalid DASH segment list.".into());
    }
    Ok(Track {
        kind: kind.into(),
        segments,
    })
}
fn dash_document(text: &str) -> Result<roxmltree::Document<'_>, String> {
    if text.len() > 4 * 1024 * 1024 {
        return Err("DASH manifest is too large.".into());
    }
    let doc = roxmltree::Document::parse(text).map_err(|_| "Invalid DASH XML manifest.")?;
    if !doc.root_element().has_tag_name("MPD") {
        return Err("The server did not return a DASH manifest.".into());
    }
    if doc
        .descendants()
        .any(|n| n.has_tag_name("ContentProtection"))
    {
        return Err(
            "This DASH stream is DRM-protected; protected downloads are unsupported.".into(),
        );
    }
    if doc
        .root_element()
        .attribute("type")
        .is_some_and(|t| t != "static")
    {
        return Err("Live DASH recording is not supported yet.".into());
    }
    Ok(doc)
}
fn dash(text: &str, url: &str, height: u32) -> Result<Plan, String> {
    let doc = dash_document(text)?;
    let root = doc.root_element();
    let periods: Vec<_> = root
        .children()
        .filter(|n| n.has_tag_name("Period"))
        .collect();
    if periods.len() != 1 {
        return Err("Multi-period DASH is not supported yet.".into());
    }
    let period = periods[0];
    let seconds = duration(
        period
            .attribute("duration")
            .or(root.attribute("mediaPresentationDuration"))
            .ok_or("Missing DASH duration.")?,
    )?;
    let all: Vec<_> = period
        .descendants()
        .filter(|n| n.has_tag_name("Representation"))
        .collect();
    let mut tracks = vec![];
    for kind in ["video", "audio"] {
        let candidates: Vec<_> = all
            .iter()
            .copied()
            .filter(|n| {
                inherited(*n, "contentType") == Some(kind)
                    || inherited(*n, "mimeType").is_some_and(|m| m.starts_with(&format!("{kind}/")))
            })
            .collect();
        if let Some(n) = select(
            &candidates,
            height,
            |n| {
                n.attribute("height")
                    .and_then(|h| h.parse().ok())
                    .unwrap_or(0)
            },
            |n| {
                n.attribute("bandwidth")
                    .and_then(|h| h.parse().ok())
                    .unwrap_or(0)
            },
        ) {
            tracks.push(representation(*n, url, seconds)?);
        }
    }
    if tracks.is_empty() {
        return Err("No supported audio/video DASH representations.".into());
    }
    Ok(Plan { tracks })
}
#[allow(dead_code)]
pub fn qualities(url: &str, kind: &str, fetch: &mut Fetch<'_>) -> Result<Vec<Quality>, String> {
    let (base, text) = fetch(url)?;
    let mut choices = if kind == "dash" {
        let doc = dash_document(&text)?;
        doc.descendants()
            .filter(|n| n.has_tag_name("Representation"))
            .filter_map(|n| {
                let height = n.attribute("height")?.parse::<u32>().ok()?;
                let bandwidth = n
                    .attribute("bandwidth")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                Some(Quality {
                    height,
                    bandwidth,
                    label: format!("{height}p"),
                })
            })
            .collect::<Vec<_>>()
    } else {
        hls_variants(&text, &base)?
            .0
            .into_iter()
            .map(|v| Quality {
                height: v.height,
                bandwidth: v.bandwidth,
                label: if v.height > 0 {
                    format!("{}p", v.height)
                } else {
                    format!("{} kbps", v.bandwidth / 1000)
                },
            })
            .collect()
    };
    choices.sort_by_key(|v| std::cmp::Reverse((v.height, v.bandwidth)));
    choices.dedup_by_key(|v| v.height);
    choices.truncate(32);
    Ok(choices)
}
