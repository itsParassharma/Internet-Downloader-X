use super::*;
#[test]
fn pause_retains_verified_segments_and_resume_does_not_publish_missing_media() {
    let dir = std::env::temp_dir().join(format!(
        "idx-stream-resume-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    ));
    let shared = Arc::new(Shared {
        paused: AtomicBool::new(false),
        stopped: AtomicBool::new(false),
        limit: AtomicU64::new(0),
        rate_clock: Mutex::new(RateClock {
            last: Instant::now(),
            tokens: 0.0,
            limit: 0,
        }),
    });
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let worker_shared = shared.clone();
    let hits = Arc::new(Mutex::new(vec![]));
    let worker_hits = hits.clone();
    let worker = thread::spawn(move || {
        for attempt in 0..3 {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = vec![];
            let mut b = [0];
            while !request.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut b).unwrap();
                request.push(b[0]);
            }
            worker_hits.lock().unwrap().push(
                String::from_utf8(request)
                    .unwrap()
                    .lines()
                    .next()
                    .unwrap()
                    .to_owned(),
            );
            if attempt == 0 {
                socket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\npart",
                    )
                    .unwrap();
            } else if attempt == 1 {
                worker_shared.paused.store(true, Ordering::SeqCst);
                let _ = socket.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nnext",
                );
            } else {
                socket
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .unwrap();
            }
        }
    });
    let spec = DownloadSpec {
        id: 1,
        url: format!("{base}/list.m3u8"),
        destination: dir.join("video.mp4"),
        connections: 1,
        retries: 0,
        speed_limit_kib: 0,
        username: String::new(),
        password: String::new(),
        referer: String::new(),
        user_agent: String::new(),
        proxy: String::new(),
        browser: Default::default(),
        stream: crate::streaming::StreamOptions {
            kind: "hls".into(),
            max_height: 0,
        },
    };
    let plan = Plan {
        tracks: vec![crate::streaming::Track {
            kind: "video".into(),
            segments: (0..2)
                .map(|i| Segment {
                    url: format!("{base}/{i}.ts"),
                    range: None,
                    encryption: None,
                })
                .collect(),
        }],
    };
    let client = make_client(&spec).unwrap();
    let (tx, _) = mpsc::channel();
    assert!(matches!(
        run_plan(
            &spec,
            &tx,
            &shared,
            &client,
            Path::new("not-run"),
            plan.clone()
        ),
        Err(Failure::Paused)
    ));
    let resume: Resume = serde_json::from_slice(
        &fs::read(sidecar_path(&spec.destination).unwrap().join("stream.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(resume.completed.len(), 1);
    assert!(!spec.destination.exists());
    shared.paused.store(false, Ordering::SeqCst);
    assert!(matches!(
        run_plan(&spec, &tx, &shared, &client, Path::new("not-run"), plan),
        Err(Failure::Fatal(_))
    ));
    worker.join().unwrap();
    let hits = hits.lock().unwrap();
    assert_eq!(hits.iter().filter(|r| r.contains("/0.ts")).count(), 1);
    assert_eq!(hits.iter().filter(|r| r.contains("/1.ts")).count(), 2);
    assert!(!spec.destination.exists());
    fs::remove_dir_all(&dir).unwrap();
}
