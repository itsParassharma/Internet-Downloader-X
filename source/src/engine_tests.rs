use super::*;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::AtomicUsize;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Ranges,
    NonRange,
    UnknownLength,
    BadEnd,
    ChangedValidator,
    TruncatedOnce,
    Empty,
}

struct Fixture {
    url: String,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
    requests: Arc<Mutex<Vec<(u64, u64)>>>,
    version: Arc<AtomicUsize>,
    data: Vec<u8>,
}

impl Fixture {
    fn new(mode: Mode, size: usize) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/test.bin", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let version = Arc::new(AtomicUsize::new(1));
        let data: Vec<u8> = (0..size).map(|n| ((n * 37 + n / 71) % 251) as u8).collect();
        let (worker_stop, worker_requests, worker_version, worker_data) = (
            stop.clone(),
            requests.clone(),
            version.clone(),
            Arc::new(data.clone()),
        );
        let fail_once = Arc::new(AtomicBool::new(mode == Mode::TruncatedOnce));
        let handle = thread::spawn(move || {
            while !worker_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let (data, requests, version, stop, fail_once) = (
                            worker_data.clone(),
                            worker_requests.clone(),
                            worker_version.clone(),
                            worker_stop.clone(),
                            fail_once.clone(),
                        );
                        thread::spawn(move || {
                            serve(stream, mode, &data, &requests, &version, &stop, &fail_once)
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            url,
            stop,
            thread: Some(handle),
            requests,
            version,
            data,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            handle.join().unwrap();
        }
    }
}

fn serve(
    mut stream: TcpStream,
    mode: Mode,
    data: &[u8],
    requests: &Mutex<Vec<(u64, u64)>>,
    version: &AtomicUsize,
    stop: &AtomicBool,
    fail_once: &AtomicBool,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut request = Vec::new();
    let mut byte = [0];
    while request.len() < 16_384 && !request.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(1) => request.push(byte[0]),
            _ => return,
        }
    }
    let request = String::from_utf8_lossy(&request);
    let range = request.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if !key.eq_ignore_ascii_case("range") {
            return None;
        }
        let (start, end) = value.trim().strip_prefix("bytes=")?.split_once('-')?;
        Some((start.parse::<u64>().ok()?, end.parse::<u64>().ok()?))
    });
    if mode == Mode::Empty {
        let _ = stream.write_all(b"HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    }
    let current_version = version.load(Ordering::Relaxed);
    let mut identity = current_version;
    let (status, start, end, ranged) = if matches!(mode, Mode::NonRange | Mode::UnknownLength) {
        ("200 OK", 0, data.len() as u64 - 1, false)
    } else if let Some((start, end)) = range {
        requests.lock().unwrap().push((start, end));
        ("206 Partial Content", start, end, true)
    } else {
        ("200 OK", 0, data.len() as u64 - 1, false)
    };
    let is_probe = range == Some((0, 0));
    if mode == Mode::ChangedValidator && !is_probe {
        identity += 1;
    }
    let mut header =
        format!("HTTP/1.1 {status}\r\nETag: \"fixture-{identity}\"\r\nConnection: close\r\n");
    if mode != Mode::UnknownLength {
        header.push_str(&format!("Content-Length: {}\r\n", end - start + 1));
    }
    if ranged {
        let response_end = if mode == Mode::BadEnd && !is_probe {
            end.saturating_sub(1)
        } else {
            end
        };
        header.push_str(&format!(
            "Content-Range: bytes {start}-{response_end}/{}\r\n",
            data.len()
        ));
    }
    header.push_str("\r\n");
    if stream.write_all(header.as_bytes()).is_err() {
        return;
    }
    let body = &data[start as usize..=end as usize];
    let body = if !is_probe && fail_once.swap(false, Ordering::SeqCst) {
        &body[..body.len() / 2]
    } else {
        body
    };
    for chunk in body.chunks(8192) {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let transformed: Vec<_> = chunk
            .iter()
            .map(|b| if current_version == 1 { *b } else { *b ^ 0x5a })
            .collect();
        if stream.write_all(&transformed).is_err() {
            return;
        }
        if !is_probe {
            thread::sleep(Duration::from_millis(1));
        }
    }
}

static TEST_ID: AtomicU64 = AtomicU64::new(1);
struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "idx-engine-test-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn spec(fixture: &Fixture, directory: &TempDir) -> DownloadSpec {
    DownloadSpec {
        browser: Default::default(),
        stream: Default::default(),
        id: 1,
        url: fixture.url.clone(),
        destination: directory.0.join("result.bin"),
        connections: 4,
        retries: 2,
        speed_limit_kib: 0,
        username: String::new(),
        password: String::new(),
        referer: String::new(),
        user_agent: String::new(),
        proxy: String::new(),
    }
}

fn terminal(receiver: &mpsc::Receiver<DownloadEvent>) -> EventKind {
    loop {
        let event = receiver
            .recv_timeout(Duration::from_secs(20))
            .expect("engine did not complete in time");
        if matches!(
            event.kind,
            EventKind::Completed { .. } | EventKind::Failed(_) | EventKind::Paused
        ) {
            return event.kind;
        }
    }
}

#[test]
fn parallel_ranges_reproduce_exact_bytes() {
    let fixture = Fixture::new(Mode::Ranges, 1_048_576);
    let directory = TempDir::new();
    let spec = spec(&fixture, &directory);
    let (sender, receiver) = mpsc::channel();
    spawn(spec.clone(), sender);
    assert!(matches!(terminal(&receiver), EventKind::Completed { .. }));
    assert_eq!(fs::read(&spec.destination).unwrap(), fixture.data);
    assert!(
        fixture
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|(start, _)| *start > 0)
    );
    assert!(!sidecar_path(&spec.destination).unwrap().exists());
}

#[test]
fn servers_without_ranges_and_without_content_length_work() {
    for mode in [Mode::NonRange, Mode::UnknownLength] {
        let fixture = Fixture::new(mode, 128_123);
        let directory = TempDir::new();
        let spec = spec(&fixture, &directory);
        let (sender, receiver) = mpsc::channel();
        spawn(spec.clone(), sender);
        assert!(matches!(terminal(&receiver), EventKind::Completed { .. }));
        assert_eq!(fs::read(&spec.destination).unwrap(), fixture.data);
    }
}

#[test]
fn pause_and_new_engine_instance_resume_existing_parts() {
    let fixture = Fixture::new(Mode::Ranges, 1_048_576);
    let directory = TempDir::new();
    let mut spec = spec(&fixture, &directory);
    spec.speed_limit_kib = 512;
    let (sender, receiver) = mpsc::channel();
    let control = spawn(spec.clone(), sender);
    loop {
        match receiver.recv_timeout(Duration::from_secs(5)).unwrap().kind {
            EventKind::Progress { downloaded, .. } if downloaded > 0 => {
                control.pause();
                break;
            }
            EventKind::Failed(message) => panic!("{message}"),
            _ => {}
        }
    }
    assert!(matches!(terminal(&receiver), EventKind::Paused));
    assert!(!spec.destination.exists());
    let manifest: Manifest = serde_json::from_slice(
        &fs::read(
            sidecar_path(&spec.destination)
                .unwrap()
                .join("manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let starts: Vec<_> = manifest.segments.iter().map(|s| s.start).collect();
    fixture.requests.lock().unwrap().clear();
    spec.speed_limit_kib = 0;
    let (sender, receiver) = mpsc::channel();
    spawn(spec.clone(), sender);
    assert!(matches!(terminal(&receiver), EventKind::Completed { .. }));
    assert_eq!(fs::read(spec.destination).unwrap(), fixture.data);
    assert!(
        fixture
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|(start, end)| start != end && !starts.contains(start))
    );
}

#[test]
fn changed_resource_after_pause_restarts_instead_of_mixing_versions() {
    let fixture = Fixture::new(Mode::Ranges, 1_048_576);
    let directory = TempDir::new();
    let mut spec = spec(&fixture, &directory);
    spec.speed_limit_kib = 512;
    let (sender, receiver) = mpsc::channel();
    let control = spawn(spec.clone(), sender);
    loop {
        if matches!(receiver.recv_timeout(Duration::from_secs(5)).unwrap().kind, EventKind::Progress { downloaded, .. } if downloaded > 0)
        {
            control.pause();
            break;
        }
    }
    assert!(matches!(terminal(&receiver), EventKind::Paused));
    fixture.version.store(2, Ordering::Relaxed);
    spec.speed_limit_kib = 0;
    let (sender, receiver) = mpsc::channel();
    spawn(spec.clone(), sender);
    assert!(matches!(terminal(&receiver), EventKind::Completed { .. }));
    let expected: Vec<_> = fixture.data.iter().map(|b| *b ^ 0x5a).collect();
    assert_eq!(fs::read(spec.destination).unwrap(), expected);
}

#[test]
fn wrong_range_and_changed_validator_never_publish_a_file() {
    for mode in [Mode::BadEnd, Mode::ChangedValidator] {
        let fixture = Fixture::new(mode, 128_000);
        let directory = TempDir::new();
        let spec = spec(&fixture, &directory);
        let (sender, receiver) = mpsc::channel();
        spawn(spec.clone(), sender);
        assert!(matches!(terminal(&receiver), EventKind::Failed(_)));
        assert!(!spec.destination.exists());
    }
}

#[test]
fn interrupted_range_retries_from_persisted_byte_offset() {
    let fixture = Fixture::new(Mode::TruncatedOnce, 512_000);
    let directory = TempDir::new();
    let spec = spec(&fixture, &directory);
    let (sender, receiver) = mpsc::channel();
    spawn(spec.clone(), sender);
    assert!(matches!(terminal(&receiver), EventKind::Completed { .. }));
    assert_eq!(fs::read(spec.destination).unwrap(), fixture.data);
}

#[test]
fn existing_destination_is_preserved() {
    let fixture = Fixture::new(Mode::Ranges, 8192);
    let directory = TempDir::new();
    let spec = spec(&fixture, &directory);
    fs::write(&spec.destination, b"an unrelated existing file").unwrap();
    let (sender, receiver) = mpsc::channel();
    spawn(spec.clone(), sender);
    assert!(matches!(terminal(&receiver), EventKind::Failed(_)));
    assert_eq!(
        fs::read(spec.destination).unwrap(),
        b"an unrelated existing file"
    );
}

#[test]
fn a_destination_created_during_download_is_never_replaced() {
    let fixture = Fixture::new(Mode::Ranges, 512_000);
    let directory = TempDir::new();
    let mut spec = spec(&fixture, &directory);
    spec.speed_limit_kib = 512;
    let (sender, receiver) = mpsc::channel();
    spawn(spec.clone(), sender);
    loop {
        match receiver.recv_timeout(Duration::from_secs(5)).unwrap().kind {
            EventKind::Started { .. } => break,
            EventKind::Failed(message) => panic!("{message}"),
            _ => {}
        }
    }
    fs::write(&spec.destination, b"created while IDX was downloading").unwrap();
    assert!(matches!(terminal(&receiver), EventKind::Failed(_)));
    assert_eq!(
        fs::read(spec.destination).unwrap(),
        b"created while IDX was downloading"
    );
}

#[test]
fn changing_speed_limit_releases_waiting_workers() {
    let fixture = Fixture::new(Mode::Ranges, 128_000);
    let directory = TempDir::new();
    let mut spec = spec(&fixture, &directory);
    spec.speed_limit_kib = 1;
    let (sender, receiver) = mpsc::channel();
    let control = spawn(spec.clone(), sender);
    loop {
        if matches!(
            receiver.recv_timeout(Duration::from_secs(5)).unwrap().kind,
            EventKind::Started { .. }
        ) {
            break;
        }
    }
    thread::sleep(Duration::from_millis(150));
    let changed = Instant::now();
    control.set_speed_limit(0);
    assert!(matches!(terminal(&receiver), EventKind::Completed { .. }));
    assert!(changed.elapsed() < Duration::from_secs(3));
    assert_eq!(fs::read(spec.destination).unwrap(), fixture.data);
}

#[test]
fn empty_file_can_be_downloaded() {
    let fixture = Fixture::new(Mode::Empty, 0);
    let directory = TempDir::new();
    let spec = spec(&fixture, &directory);
    let (sender, receiver) = mpsc::channel();
    spawn(spec.clone(), sender);
    assert!(matches!(terminal(&receiver), EventKind::Completed { .. }));
    assert!(fs::read(spec.destination).unwrap().is_empty());
}

#[cfg(windows)]
#[test]
fn extended_length_destination_publishes_exact_bytes_without_replacing_files() {
    use std::os::windows::ffi::OsStrExt;
    let fixture = Fixture::new(Mode::Ranges, 64_123);
    let directory = TempDir::new();
    let mut nested = directory.0.clone();
    while nested.as_os_str().encode_wide().count() <= 300 {
        nested.push("nested-download-folder-with-a-long-name-0123456789");
    }
    let mut spec = spec(&fixture, &directory);
    spec.destination = nested.join("long-path-result.bin");
    assert!(spec.destination.as_os_str().encode_wide().count() > 260);
    let (sender, receiver) = mpsc::channel();
    spawn(spec.clone(), sender);
    let result = terminal(&receiver);
    assert!(matches!(result, EventKind::Completed { .. }), "{result:?}");
    assert_eq!(fs::read(&spec.destination).unwrap(), fixture.data);

    let competing_file = nested.join("competing-download.tmp");
    fs::write(&competing_file, b"must not replace the completed file").unwrap();
    assert!(publish_file(&competing_file, &spec.destination).is_err());
    assert_eq!(fs::read(&spec.destination).unwrap(), fixture.data);
    assert_eq!(
        fs::read(&competing_file).unwrap(),
        b"must not replace the completed file"
    );
}
