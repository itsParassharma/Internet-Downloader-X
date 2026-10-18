//! Rust segment transfer, resumable verified parts, and local-only stream-copy muxing.
use super::*;
use crate::streaming::{Plan, Segment};
use std::process::{Command, Stdio};
#[cfg(test)]
#[path = "stream_download_tests.rs"]
mod tests;

fn fetch(
    client: &Client,
    spec: &DownloadSpec,
    url: &str,
    range: Option<(u64, u64)>,
    limit: usize,
    shared: &Shared,
) -> Result<Vec<u8>, Failure> {
    let mut attempt = 0;
    loop {
        shared.check()?;
        let result = (|| {
            let mut target = spec.clone();
            target.url = url.into();
            let mut builder = request(client, &target);
            if let Some((s, e)) = range {
                builder = builder.header(RANGE, format!("bytes={s}-{e}"));
            }
            let mut response = crate::browser_session::send(client, builder, &spec.browser)
                .map_err(Failure::Retry)?;
            if !response.status().is_success() {
                return Err(status_error(response.status()));
            }
            identity_encoding(&response)?;
            if let Some((s, e)) = range {
                let actual = response
                    .headers()
                    .get(CONTENT_RANGE)
                    .and_then(|h| h.to_str().ok())
                    .and_then(parse_range);
                if response.status().as_u16() != 206
                    || actual.is_none_or(|(a, b, _)| (a, b) != (s, e))
                {
                    return Err(Failure::Fatal(
                        "The media server returned an incorrect byte range.".into(),
                    ));
                }
            }
            let expected = content_length(&response)?;
            if expected.is_some_and(|n| n > limit as u64) {
                return Err(Failure::Fatal(
                    "The media resource exceeds the supported size.".into(),
                ));
            }
            let mut result = Vec::new();
            let mut buf = [0; 65536];
            loop {
                shared.check()?;
                let n = response.read(&mut buf).map_err(|_| {
                    Failure::Retry("The media segment connection was interrupted.".into())
                })?;
                if n == 0 {
                    break;
                }
                if result.len() + n > limit {
                    return Err(Failure::Fatal(
                        "The media resource exceeds the supported size.".into(),
                    ));
                }
                shared.throttle(n)?;
                result.extend_from_slice(&buf[..n]);
            }
            if expected.is_some_and(|n| n != result.len() as u64)
                || range.is_some_and(|(s, e)| e - s + 1 != result.len() as u64)
            {
                return Err(Failure::Retry("The media segment is incomplete.".into()));
            }
            Ok(result)
        })();
        match result {
            Err(Failure::Retry(_)) if attempt < spec.retries => {
                attempt += 1;
                shared.delay(Duration::from_millis(250 * 2u64.pow(attempt.min(5))))?;
            }
            result => return result,
        }
    }
}
fn fetch_file(
    client: &Client,
    spec: &DownloadSpec,
    segment: &Segment,
    path: &Path,
    shared: &Shared,
    mut progress: impl FnMut(u64),
) -> Result<Part, Failure> {
    let mut attempt = 0;
    loop {
        shared.check()?;
        let result = (|| {
            let mut target = spec.clone();
            target.url = segment.url.clone();
            let mut builder = request(client, &target);
            if let Some((s, e)) = segment.range {
                builder = builder.header(RANGE, format!("bytes={s}-{e}"));
            }
            let mut response = crate::browser_session::send(client, builder, &spec.browser)
                .map_err(Failure::Retry)?;
            if !response.status().is_success() {
                return Err(status_error(response.status()));
            }
            identity_encoding(&response)?;
            if let Some((s, e)) = segment.range {
                let actual = response
                    .headers()
                    .get(CONTENT_RANGE)
                    .and_then(|h| h.to_str().ok())
                    .and_then(parse_range);
                if response.status().as_u16() != 206
                    || actual.is_none_or(|(a, b, _)| (a, b) != (s, e))
                {
                    return Err(Failure::Fatal(
                        "The media server returned an incorrect byte range.".into(),
                    ));
                }
            }
            let expected = content_length(&response)?;
            let mut output =
                File::create(path).map_err(|e| io_error("Could not create media part", e))?;
            let mut hash = Sha256::new();
            let mut bytes = 0u64;
            let mut buf = [0; 65536];
            let mut tick = Instant::now();
            loop {
                shared.check()?;
                let n = response
                    .read(&mut buf)
                    .map_err(|_| Failure::Retry("The media connection was interrupted.".into()))?;
                if n == 0 {
                    break;
                }
                shared.throttle(n)?;
                output
                    .write_all(&buf[..n])
                    .map_err(|e| io_error("Could not save media data", e))?;
                hash.update(&buf[..n]);
                bytes += n as u64;
                if tick.elapsed() >= Duration::from_millis(200) {
                    progress(bytes);
                    tick = Instant::now();
                }
            }
            if expected.is_some_and(|n| n != bytes)
                || segment.range.is_some_and(|(s, e)| e - s + 1 != bytes)
            {
                return Err(Failure::Retry("The media resource is incomplete.".into()));
            }
            output
                .sync_all()
                .map_err(|e| io_error("Could not flush media data", e))?;
            Ok(Part {
                hash: format!("{:x}", hash.finalize()),
                length: bytes,
            })
        })();
        match result {
            Err(Failure::Retry(_)) if attempt < spec.retries => {
                attempt += 1;
                shared.delay(Duration::from_millis(250 * 2u64.pow(attempt.min(5))))?;
            }
            result => return result,
        }
    }
}
pub(super) fn text(
    client: &Client,
    spec: &DownloadSpec,
    url: &str,
    shared: &Shared,
) -> Result<(String, String), String> {
    // Keep the final response URL so relative segment references follow redirects.
    shared.check().map_err(Failure::message)?;
    let mut target = spec.clone();
    target.url = url.into();
    let mut response =
        crate::browser_session::send(client, request(client, &target), &spec.browser)?;
    if !response.status().is_success() {
        return Err(status_error(response.status()).message());
    }
    let base = response.url().to_string();
    let mut bytes = vec![];
    response
        .by_ref()
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read the media manifest.")?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("The media manifest is too large.".into());
    }
    String::from_utf8(bytes)
        .map(|text| (base, text))
        .map_err(|_| "The media manifest is not UTF-8.".into())
}
#[derive(Serialize, Deserialize, Default)]
struct Resume {
    plan_hash: String,
    completed: Vec<Part>,
}
#[derive(Serialize, Deserialize)]
struct Part {
    hash: String,
    length: u64,
}
fn save_resume(dir: &Path, resume: &Resume) -> Result<(), Failure> {
    let temp = dir.join("stream.tmp");
    safe_regular(&temp)?;
    let mut f = File::create(&temp).map_err(|e| io_error("Could not save stream progress", e))?;
    f.write_all(&serde_json::to_vec(resume).unwrap())
        .and_then(|_| f.sync_all())
        .map_err(|e| io_error("Could not save stream progress", e))?;
    fs::rename(temp, dir.join("stream.json"))
        .map_err(|e| io_error("Could not commit stream progress", e))
}
fn digest_file(path: &Path) -> Result<String, Failure> {
    let mut file = File::open(path).map_err(|e| io_error("Could not inspect media part", e))?;
    let mut hash = Sha256::new();
    let mut b = [0; 65536];
    loop {
        let n = file
            .read(&mut b)
            .map_err(|e| io_error("Could not verify media part", e))?;
        if n == 0 {
            break;
        }
        hash.update(&b[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn helper() -> Result<PathBuf, Failure> {
    let exe = std::env::current_exe().map_err(|e| io_error("Could not locate media helper", e))?;
    let filename = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    let mut candidates = vec![
        exe.parent()
            .unwrap_or(Path::new("."))
            .join("media-tools")
            .join(filename),
    ];
    // Explicit development override; release packages always use the bundled helper.
    if cfg!(debug_assertions) {
        candidates.push(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("media-tools")
                .join(filename),
        );
    }
    candidates.into_iter().find(|p| p.is_file()).ok_or_else(|| {
        Failure::Fatal("The media assembly helper is missing. Reinstall IDX.".into())
    })
}
fn demuxer(path: &Path) -> Result<&'static str, Failure> {
    let mut bytes = [0; 376];
    let n = File::open(path)
        .and_then(|mut f| f.read(&mut bytes))
        .map_err(|e| io_error("Could not inspect media format", e))?;
    if n >= 8 && matches!(&bytes[4..8], b"ftyp" | b"styp" | b"moov") {
        Ok("mov")
    } else if n >= 188 && bytes[0] == 0x47 {
        Ok("mpegts")
    } else if n >= 4 && bytes[..4] == [0x1a, 0x45, 0xdf, 0xa3] {
        Ok("matroska")
    } else if n >= 2 && bytes[0] == 0xff && bytes[1] & 0xf6 == 0xf0 {
        Ok("aac")
    } else {
        Err(Failure::Fatal(
            "The downloaded segments are not a supported media container.".into(),
        ))
    }
}
pub(super) fn run(
    spec: &DownloadSpec,
    sender: &Sender<DownloadEvent>,
    shared: &Arc<Shared>,
) -> Result<(), Failure> {
    let helper = helper()?;
    let client = make_client(spec)?;
    let plan = crate::streaming::plan(&spec.url, &spec.stream, &mut |u| {
        text(&client, spec, u, shared)
    })
    .map_err(|e| {
        if shared.interrupted() {
            Failure::Paused
        } else {
            Failure::Fatal(e)
        }
    })?;
    run_plan(spec, sender, shared, &client, &helper, plan)
}
fn run_plan(
    spec: &DownloadSpec,
    sender: &Sender<DownloadEvent>,
    shared: &Arc<Shared>,
    client: &Client,
    helper: &Path,
    plan: Plan,
) -> Result<(), Failure> {
    if spec.destination.exists() {
        return Err(Failure::Fatal("The destination already exists.".into()));
    }
    if let Some(parent) = spec.destination.parent() {
        fs::create_dir_all(parent).map_err(|e| io_error("Could not create download folder", e))?;
    }
    let directory = sidecar_path(&spec.destination)?;
    if fs::symlink_metadata(&directory).is_ok_and(|m| !m.file_type().is_dir()) {
        return Err(Failure::Fatal(
            "The media resume path is not a regular folder.".into(),
        ));
    }
    fs::create_dir_all(&directory)
        .map_err(|e| io_error("Could not create media resume folder", e))?;
    let lock = acquire_lock(&directory)?;
    let manifest = directory.join("stream.json");
    safe_regular(&manifest)?;
    let hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&plan).unwrap()));
    let segments: Vec<&Segment> = plan.tracks.iter().flat_map(|t| &t.segments).collect();
    let mut resume = fs::read(&manifest)
        .ok()
        .filter(|v| v.len() < 16 * 1024 * 1024)
        .and_then(|b| serde_json::from_slice::<Resume>(&b).ok())
        .filter(|r| r.plan_hash == hash && r.completed.len() <= segments.len())
        .unwrap_or(Resume {
            plan_hash: hash,
            completed: vec![],
        });
    // Verify cached segments before trusting resume state; a modified part is fetched again.
    let valid = resume
        .completed
        .iter()
        .enumerate()
        .take_while(|(i, p)| {
            let path = directory.join(format!("media-{i}.part"));
            safe_regular(&path).is_ok()
                && fs::metadata(&path).is_ok_and(|m| m.len() == p.length)
                && digest_file(&path).is_ok_and(|h| h == p.hash)
        })
        .count();
    resume.completed.truncate(valid);
    save_resume(&directory, &resume)?;
    let send = |kind| {
        let _ = sender.send(DownloadEvent { id: spec.id, kind });
    };
    send(EventKind::Started {
        total: None,
        resumable: true,
    });
    let started = Instant::now();
    let mut bytes: u64 = resume.completed.iter().map(|p| p.length).sum();
    let initial = bytes;
    let mut keys = std::collections::HashMap::<String, Vec<u8>>::new();
    for (index, segment) in segments.iter().enumerate().skip(valid) {
        shared.check()?;
        let path = directory.join(format!("media-{index}.part"));
        let temp = directory.join("media.tmp");
        safe_regular(&path)?;
        safe_regular(&temp)?;
        let part = if segment.encryption.is_some() {
            let mut body = fetch(
                client,
                spec,
                &segment.url,
                segment.range,
                256 * 1024 * 1024,
                shared,
            )?;
            {
                let encryption = segment.encryption.as_ref().unwrap();
                if !keys.contains_key(&encryption.url) {
                    let key = fetch(client, spec, &encryption.url, None, 16, shared)?;
                    if key.len() != 16 {
                        return Err(Failure::Fatal("Invalid HLS AES-128 key length.".into()));
                    }
                    keys.insert(encryption.url.clone(), key);
                }
                use cbc::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
                let cipher = cbc::Decryptor::<aes::Aes128>::new_from_slices(
                    &keys[&encryption.url],
                    &encryption.iv,
                )
                .map_err(|_| Failure::Fatal("Invalid HLS encryption parameters.".into()))?;
                let size = cipher
                    .decrypt_padded_mut::<Pkcs7>(&mut body)
                    .map_err(|_| Failure::Fatal("The HLS segment could not be decrypted.".into()))?
                    .len();
                body.truncate(size);
            }
            let mut file =
                File::create(&temp).map_err(|e| io_error("Could not write media segment", e))?;
            file.write_all(&body)
                .and_then(|_| file.sync_all())
                .map_err(|e| io_error("Could not save media segment", e))?;
            Part {
                hash: format!("{:x}", Sha256::digest(&body)),
                length: body.len() as u64,
            }
        } else {
            fetch_file(client, spec, segment, &temp, shared, |current| {
                send(EventKind::Progress {
                    downloaded: bytes + current,
                    total: None,
                    speed: (bytes + current).saturating_sub(initial) as f64
                        / started.elapsed().as_secs_f64().max(0.01),
                    parts: vec![PartProgress {
                        downloaded: index as u64,
                        total: segments.len() as u64,
                    }],
                })
            })?
        };
        fs::rename(&temp, &path).map_err(|e| io_error("Could not commit media segment", e))?;
        bytes += part.length;
        resume.completed.push(part);
        save_resume(&directory, &resume)?;
        send(EventKind::Progress {
            downloaded: bytes,
            total: None,
            speed: bytes.saturating_sub(initial) as f64 / started.elapsed().as_secs_f64().max(0.01),
            parts: vec![PartProgress {
                downloaded: (index + 1) as u64,
                total: segments.len() as u64,
            }],
        });
    }
    let mut offset = 0;
    let mut tracks = vec![];
    for (index, track) in plan.tracks.iter().enumerate() {
        let path = directory.join(format!("track-{index}.bin"));
        safe_regular(&path)?;
        let mut out =
            File::create(&path).map_err(|e| io_error("Could not assemble media track", e))?;
        for i in offset..offset + track.segments.len() {
            shared.check()?;
            let mut part = File::open(directory.join(format!("media-{i}.part")))
                .map_err(|e| io_error("Could not read media segment", e))?;
            let mut buf = [0; 65536];
            loop {
                shared.check()?;
                let n = part
                    .read(&mut buf)
                    .map_err(|e| io_error("Could not read media segment", e))?;
                if n == 0 {
                    break;
                }
                out.write_all(&buf[..n])
                    .map_err(|e| io_error("Could not assemble media track", e))?;
            }
        }
        out.sync_all()
            .map_err(|e| io_error("Could not flush media track", e))?;
        offset += track.segments.len();
        tracks.push(path);
    }
    shared.check()?;
    let assembled = directory.join("assembled.mp4");
    safe_regular(&assembled)?;
    let mut command = Command::new(helper);
    command.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"]);
    for path in &tracks {
        command
            .args([
                "-protocol_whitelist",
                "file,pipe",
                "-f",
                demuxer(path)?,
                "-i",
            ])
            .arg(path);
    }
    for (i, track) in plan.tracks.iter().enumerate() {
        if track.kind == "audio" {
            command.args(["-map", &format!("{i}:a:0?")]);
        } else {
            command.args(["-map", &format!("{i}:v:0?")]);
            if plan.tracks.len() == 1 {
                command.args(["-map", &format!("{i}:a:0?")]);
            }
        }
    }
    command
        .args(["-c", "copy", "-movflags", "+faststart", "-f", "mp4"])
        .arg(&assembled)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .map_err(|_| Failure::Fatal("Could not start the bundled media assembly helper.".into()))?;
    loop {
        if shared.interrupted() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Failure::Paused);
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|e| io_error("Could not check media assembly", e))?
        {
            if !status.success() {
                return Err(Failure::Fatal(
                    "Media assembly failed. The downloaded segments were retained for retry."
                        .into(),
                ));
            }
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    shared.check()?;
    let completed = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&assembled)
        .map_err(|e| io_error("Missing assembled video", e))?;
    let length = completed
        .metadata()
        .map_err(|e| io_error("Could not check video", e))?
        .len();
    if length == 0 {
        return Err(Failure::Fatal(
            "Media assembly produced an empty video.".into(),
        ));
    }
    completed
        .sync_all()
        .map_err(|e| io_error("Could not flush video", e))?;
    drop(completed);
    publish_file(&assembled, &spec.destination).map_err(|e| {
        io_error(
            "Could not publish completed video; an existing file was preserved",
            e,
        )
    })?;
    send(EventKind::Progress {
        downloaded: length,
        total: Some(length),
        speed: 0.0,
        parts: vec![],
    });
    drop(lock);
    for i in 0..segments.len() {
        let _ = fs::remove_file(directory.join(format!("media-{i}.part")));
    }
    for path in tracks {
        let _ = fs::remove_file(path);
    }
    for name in ["assembled.mp4", "stream.json", "active.lock"] {
        let _ = fs::remove_file(directory.join(name));
    }
    let _ = fs::remove_dir(directory);
    Ok(())
}
