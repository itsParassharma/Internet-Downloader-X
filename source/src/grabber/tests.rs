use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::Instant,
};

struct Fixture {
    origin: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
fn request(stream: &mut TcpStream) -> String {
    request_line(stream).1
}
fn request_line(stream: &mut TcpStream) -> (String, String) {
    // Windows accepted sockets inherit the listener's nonblocking mode.
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(4)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut buffer = [0; 1024];
    while !bytes.windows(4).any(|s| s == b"\r\n\r\n") {
        let size = stream.read(&mut buffer).unwrap();
        assert!(size > 0 && bytes.len() < 16384);
        bytes.extend_from_slice(&buffer[..size]);
    }
    let text = String::from_utf8(bytes).unwrap();
    let mut words = text.split_whitespace();
    (words.next().unwrap().into(), words.next().unwrap().into())
}
fn html(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}
fn redirect(to: &str) -> String {
    format!(
        "HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}
impl Fixture {
    fn new(route: impl Fn(&str, &str) -> String + Send + 'static) -> Self {
        Self::with_methods(move |base, _, path| route(base, path))
    }
    fn with_methods(route: impl Fn(&str, &str, &str) -> String + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (logs, done, base) = (requests.clone(), stop.clone(), origin.clone());
        let thread = std::thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let (method, path) = request_line(&mut stream);
                        logs.lock().unwrap().push(path.clone());
                        stream
                            .set_write_timeout(Some(Duration::from_secs(3)))
                            .unwrap();
                        let _ = stream.write_all(route(&base, &method, &path).as_bytes());
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(3))
                    }
                    Err(e) => panic!("{e}"),
                }
            }
        });
        Self {
            origin,
            requests,
            stop,
            thread: Some(thread),
        }
    }
    fn paths(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
    fn scan(&self, path: &str, options: ScanOptions) -> PageLinks {
        let (tx, rx) = mpsc::channel();
        let _control = scan(format!("{}{path}", self.origin), options, tx);
        finished(&rx).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let result = self.thread.take().unwrap().join();
        if !std::thread::panicking() {
            result.unwrap();
        }
    }
}
fn finished(rx: &mpsc::Receiver<ScanEvent>) -> Result<PageLinks, String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap()
        {
            ScanEvent::Progress { .. } | ScanEvent::Checking { .. } => {}
            ScanEvent::Finished(result) => return result,
        }
    }
}

#[test]
fn redirect_keeps_resolved_source_page_with_its_links() {
    let server = Fixture::new(|_, path| match path {
        "/start" => redirect("/resolved/index.html"),
        "/resolved/index.html" => html(r#"<a href="file.zip">File</a><img src="image.png">"#),
        _ => panic!("Unexpected request {path}"),
    });
    let result = server.scan("/start", ScanOptions::default());
    assert_eq!(
        result.page_url,
        format!("{}/resolved/index.html", server.origin)
    );
    assert_eq!(
        result.urls,
        vec![
            format!("{}/resolved/file.zip", server.origin),
            format!("{}/resolved/image.png", server.origin)
        ]
    );
    assert!(result.parents.values().all(|page| page == &result.page_url));
}

#[test]
fn recursive_scan_obeys_depth_breaks_cycles_and_preserves_per_link_parent() {
    let server = Fixture::new(|_, path| {
        html(match path {
            "/index.html" => {
                "<a href='child.html#one'>Child</a><a href='child.html#two'>Again</a><a href='root.zip'>Root</a>"
            }
            "/child.html" => {
                "<a href='index.html'>Cycle</a><a href='deep.html'>Deep</a><img src='child.png'>"
            }
            "/deep.html" => "<a href='deep.zip'>Deep file</a>",
            _ => panic!("File should not be fetched: {path}"),
        })
    });
    let one = server.scan(
        "/index.html",
        ScanOptions {
            levels: 1,
            ..Default::default()
        },
    );
    assert_eq!(server.paths(), ["/index.html", "/child.html"]);
    assert_eq!(one.pages, 2);
    assert!(!one.urls.iter().any(|u| u.ends_with("deep.zip")));
    assert_eq!(
        one.parents[&format!("{}/child.png", server.origin)],
        format!("{}/child.html", server.origin)
    );
    let two = server.scan(
        "/index.html",
        ScanOptions {
            levels: 2,
            ..Default::default()
        },
    );
    assert_eq!(two.pages, 3);
    assert!(two.urls.iter().any(|u| u.ends_with("deep.zip")));
}

#[test]
fn external_links_and_redirects_stay_outside_scan_unless_enabled() {
    let outside = Fixture::new(|_, path| {
        html(match path {
            "/outside.html" => "<a href='more.html'>More</a><a href='outside.zip'>File</a>",
            _ => panic!("External depth exceeded: {path}"),
        })
    });
    let external = outside.origin.replace("127.0.0.1", "localhost");
    let server = Fixture::new(move |_, path| match path {
        "/index.html" => html(&format!(
            "<a href='{external}/outside.html'>External</a><a href='redirect.html'>Redirect</a>"
        )),
        "/redirect.html" => redirect(&format!("{external}/outside.html")),
        _ => panic!("{path}"),
    });
    let local = server.scan(
        "/index.html",
        ScanOptions {
            levels: 2,
            ..Default::default()
        },
    );
    assert_eq!(local.pages, 1);
    assert!(outside.paths().is_empty());
    let external = server.scan(
        "/index.html",
        ScanOptions {
            levels: 2,
            other_levels: 1,
            ..Default::default()
        },
    );
    assert_eq!(external.pages, 2);
    assert_eq!(outside.paths(), ["/outside.html"]);
    assert!(external.urls.iter().any(|u| u.ends_with("outside.zip")));
}

#[test]
fn external_redirect_back_to_base_cannot_bypass_internal_depth() {
    let outside_address = Arc::new(Mutex::new(String::new()));
    let outside_for_page = outside_address.clone();
    let server = Fixture::new(move |_, path| {
        assert_eq!(
            path, "/index.html",
            "Redirect bypassed the internal level limit"
        );
        html(&format!(
            "<a href='{}/outside.html'>External</a>",
            outside_for_page.lock().unwrap()
        ))
    });
    let base = server.origin.clone();
    let outside = Fixture::new(move |_, _| redirect(&format!("{base}/forbidden.html")));
    *outside_address.lock().unwrap() = outside.origin.replace("127.0.0.1", "localhost");
    let result = server.scan(
        "/index.html",
        ScanOptions {
            other_levels: 1,
            ..Default::default()
        },
    );
    assert_eq!(result.pages, 1);
    assert_eq!(result.skipped, 1);
    assert_eq!(outside.paths(), ["/outside.html"]);
}

#[test]
fn parent_directory_rule_applies_to_redirect_targets_too() {
    let server = Fixture::new(|_, path| match path {
        "/folder/index.html" => html(
            "<a href='../parent.html'>Up</a><a href='child.html'>Child</a><a href='escape.html'>Escape</a>",
        ),
        "/folder/child.html" => html("<img src='child.png'>"),
        "/folder/escape.html" => redirect("/parent.html"),
        _ => panic!("Parent directory escaped: {path}"),
    });
    let result = server.scan(
        "/folder/index.html",
        ScanOptions {
            levels: 10,
            no_parent_directories: true,
            ..Default::default()
        },
    );
    assert_eq!(result.pages, 2);
    assert!(
        result.urls.iter().any(|u| u.ends_with("parent.html")),
        "Exploration restriction must not discard downloadable links"
    );
}

#[test]
fn request_budget_includes_redirects_and_keeps_collected_results() {
    let server = Fixture::new(|_, path| match path {
        "/start" => redirect("/index.html"),
        "/index.html" => html("<a href='one.html'>One</a><a href='two.html'>Two</a>"),
        "/one.html" => html("<a href='one.zip'>File</a>"),
        _ => panic!("Budget exceeded: {path}"),
    });
    let result = server.scan(
        "/start",
        ScanOptions {
            levels: usize::MAX,
            max_requests: 3,
            ..Default::default()
        },
    );
    assert_eq!(server.paths().len(), 3);
    assert!(result.limited);
    assert!(result.urls.iter().any(|u| u.ends_with("one.zip")));
}

#[test]
fn html_base_entities_unquoted_attributes_and_ignored_scripts() {
    let parser = HtmlLinks::new();
    let page = Url::parse("https://example.com/start/index.html").unwrap();
    let links = parser.extract(
        &page,
        r#"<base href='/files/'><a href=child.html>Child</a>
      <a href='file.zip?a=1&amp;b=&#50;'>File</a><img src='image.png'>
      <!-- <a href='fake.html'> --><script>"<a href='script.html'>"</script>
      <a data-href='ignored.html' href='javascript:alert(1)'>No</a><a href='#local'>Local</a>"#,
    );
    assert_eq!(links.len(), 3);
    assert_eq!(
        links[0],
        (
            Url::parse("https://example.com/files/child.html").unwrap(),
            true
        )
    );
    assert_eq!(
        links[1].0.as_str(),
        "https://example.com/files/file.zip?a=1&b=2"
    );
    assert!(!links[2].1);
}

#[test]
fn child_failures_and_non_html_do_not_discard_good_results() {
    let server = Fixture::new(|_, path| {
        match path {
        "/index.html" => html("<a href='missing.html'>Missing</a><a href='file'>Binary</a><a href='good.html'>Good</a>"),
        "/missing.html" => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
        "/file" => "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
        "/good.html" => html("<img src='good.png'>"),
        _ => panic!("{path}"),
    }
    });
    let result = server.scan(
        "/index.html",
        ScanOptions {
            levels: 1,
            ..Default::default()
        },
    );
    assert_eq!(result.pages, 2);
    assert_eq!(result.skipped, 1);
    assert!(result.urls.iter().any(|u| u.ends_with("good.png")));
}

#[test]
fn cancel_interrupts_stalled_headers_and_body_without_waiting_for_timeout() {
    for body_started in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}/", listener.local_addr().unwrap());
        let (ready, wait) = mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            request(&mut stream);
            if body_started {
                stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 100000\r\n\r\n<a").unwrap();
            }
            ready.send(()).unwrap();
            let mut b = [0];
            // A dropped request closes its connection, including during a body read.
            let result = stream.read(&mut b);
            assert!(
                matches!(result, Ok(0))
                    || result.is_err_and(|e| e.kind() == std::io::ErrorKind::ConnectionReset)
            );
        });
        let (tx, rx) = mpsc::channel();
        let control = scan(address, ScanOptions::default(), tx);
        wait.recv_timeout(Duration::from_secs(5)).unwrap();
        let start = Instant::now();
        drop(control);
        assert_eq!(finished(&rx).unwrap_err(), "Scan cancelled.");
        assert!(start.elapsed() < Duration::from_secs(2));
        server.join().unwrap();
    }
}

#[test]
fn oversized_page_and_redirect_loop_fail_with_a_clear_error() {
    for loop_redirect in [false, true] {
        let server = Fixture::new(move |_, _| {
            if loop_redirect {
                redirect("/")
            } else {
                html(&"x".repeat(MAX_PAGE_BYTES + 1))
            }
        });
        let (tx, rx) = mpsc::channel();
        let _control = scan(format!("{}/", server.origin), ScanOptions::default(), tx);
        let error = finished(&rx).unwrap_err();
        assert!(error.contains(if loop_redirect {
            "redirect loop"
        } else {
            "4 MiB"
        }));
    }
}

fn head(size: Option<u64>) -> String {
    format!(
        "HTTP/1.1 200 OK\r\n{}Connection: close\r\n\r\n",
        size.map(|n| format!("Content-Length: {n}\r\n"))
            .unwrap_or_default()
    )
}
#[test]
fn start_page_mask_collects_the_resolved_start_page_without_a_self_link() {
    let server = Fixture::new(|_, path| match path {
        "/" => redirect("/index.html"),
        "/index.html" => html("<a href='/child.html'>Page</a><img src='/image.jpg'>"),
        _ => panic!("{path}"),
    });
    let result = server.scan(
        "/",
        ScanOptions {
            files: FileRules {
                include: "<start page>".into(),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert_eq!(result.urls, [format!("{}/index.html", server.origin)]);
    assert_eq!(result.parents[&result.urls[0]], result.page_url);
}
#[test]
fn page_path_filters_apply_before_fetch_and_after_redirect_without_restricting_file_results() {
    let server = Fixture::new(|_, path| match path {
        "/index.html" => html(
            "<a href='/allowed/child.html'>Child</a><a href='/blocked.html'>Blocked</a><a href='/allowed/escape.html'>Redirect</a><a href='/files/root.zip'>File</a>",
        ),
        "/allowed/child.html" => html("<a href='/files/child.zip'>File</a>"),
        "/allowed/escape.html" => redirect("/blocked.html"),
        _ => panic!("Excluded path fetched: {path}"),
    });
    let result = server.scan(
        "/index.html",
        ScanOptions {
            levels: 3,
            page_paths: PathRules {
                include: "/allowed/".into(),
                exclude: String::new(),
            },
            files: FileRules {
                include: "*.zip".into(),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert_eq!(result.pages, 2);
    assert_eq!(
        result.urls,
        [
            format!("{}/files/root.zip", server.origin),
            format!("{}/files/child.zip", server.origin)
        ]
    );
    assert_eq!(
        result.parents[&result.urls[1]],
        format!("{}/allowed/child.html", server.origin)
    );
}
#[test]
fn size_filters_use_head_only_reject_unknowns_and_skip_excluded_hosts_and_paths() {
    let server = Fixture::with_methods(|base, method, path| match (method, path) {
        ("GET", "/") => html(&format!(
            "<a href='/good.zip'>A</a><a href='/small.zip'>B</a><a href='/large.zip'>C</a><a href='/unknown.zip'>D</a><a href='/bad.zip'>E</a><a href='/excluded/file.zip'>F</a><a href='{}/remote.zip'>G</a><img src='/ignore.png'>",
            base.replace("127.0.0.1", "localhost")
        )),
        ("HEAD", "/good.zip") => head(Some(20)),
        ("HEAD", "/small.zip") => head(Some(9)),
        ("HEAD", "/large.zip") => head(Some(21)),
        ("HEAD", "/unknown.zip") => head(None),
        ("HEAD", "/bad.zip") => {
            "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .into()
        }
        _ => panic!("Unexpected metadata or body request: {method} {path}"),
    });
    let result = server.scan(
        "/",
        ScanOptions {
            files: FileRules {
                include: "*.zip".into(),
                site_only: true,
                paths: PathRules {
                    exclude: "/excluded/".into(),
                    ..Default::default()
                },
                min_size: Some(10),
                max_size: Some(20),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert_eq!(result.urls, [format!("{}/good.zip", server.origin)]);
    assert_eq!(result.unknown_sizes, 2);
    assert_eq!(server.paths().len(), 6);
}
#[test]
fn duplicate_filter_keeps_first_name_and_size_match_but_not_unknown_sizes_or_different_sizes() {
    let server = Fixture::with_methods(|_, method, path| match (method, path) {
        ("GET", "/") => html(
            "<a href='/first/file.zip'>1</a><a href='/second/FILE.zip'>2</a><a href='/third/file.zip'>3</a><a href='/unknown/file.zip'>4</a><a href='/unknown2/file.zip'>5</a>",
        ),
        ("HEAD", "/first/file.zip" | "/second/FILE.zip") => head(Some(100)),
        ("HEAD", "/third/file.zip") => head(Some(101)),
        ("HEAD", "/unknown/file.zip" | "/unknown2/file.zip") => head(None),
        _ => panic!("{method} {path}"),
    });
    let result = server.scan(
        "/",
        ScanOptions {
            files: FileRules {
                hide_duplicates: true,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert_eq!(result.urls.len(), 4);
    assert_eq!(result.duplicates, 1);
    assert_eq!(result.unknown_sizes, 2);
    assert!(result.urls[0].ends_with("/first/file.zip"));
    assert!(!result.urls.iter().any(|u| u.contains("/second/")));
    let plain = server.scan("/", ScanOptions::default());
    assert_eq!(plain.urls.len(), 5);
    assert_eq!(
        server.paths().len(),
        7,
        "Disabled dedup must not probe file metadata"
    );
}
#[test]
fn metadata_redirects_obey_paths_hosts_and_request_budget() {
    let server = Fixture::with_methods(|base, method, path| match (method, path) {
        ("GET", "/") => html(
            "<a href='/escape.zip'>A</a><a href='/outside.zip'>B</a><a href='/good.zip'>C</a><a href='/limit.zip'>D</a>",
        ),
        ("HEAD", "/escape.zip") => redirect("/excluded/file.zip"),
        ("HEAD", "/outside.zip") => redirect(&format!(
            "{}/other.zip",
            base.replace("127.0.0.1", "localhost")
        )),
        ("HEAD", "/good.zip") => head(Some(5)),
        _ => panic!("Scope or budget escaped: {method} {path}"),
    });
    let result = server.scan(
        "/",
        ScanOptions {
            max_requests: 3,
            files: FileRules {
                site_only: true,
                min_size: Some(1),
                paths: PathRules {
                    exclude: "/excluded/".into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert_eq!(result.urls, [format!("{}/good.zip", server.origin)]);
    assert!(result.limited);
    assert_eq!(result.unknown_sizes, 1);
    assert_eq!(server.paths().len(), 4);
}
#[test]
fn cancel_interrupts_stalled_metadata_request() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/", listener.local_addr().unwrap());
    let (ready, wait) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        assert_eq!(request_line(&mut stream), ("GET".into(), "/".into()));
        stream
            .write_all(html("<a href='/file.zip'>File</a>").as_bytes())
            .unwrap();
        drop(stream);
        let (mut stream, _) = listener.accept().unwrap();
        assert_eq!(
            request_line(&mut stream),
            ("HEAD".into(), "/file.zip".into())
        );
        ready.send(()).unwrap();
        let mut b = [0];
        let result = stream.read(&mut b);
        assert!(
            matches!(result, Ok(0))
                || result.is_err_and(|e| e.kind() == std::io::ErrorKind::ConnectionReset)
        );
    });
    let (tx, rx) = mpsc::channel();
    let control = scan(
        address,
        ScanOptions {
            files: FileRules {
                min_size: Some(1),
                ..Default::default()
            },
            ..Default::default()
        },
        tx,
    );
    wait.recv_timeout(Duration::from_secs(5)).unwrap();
    let start = Instant::now();
    drop(control);
    assert_eq!(finished(&rx).unwrap_err(), "Scan cancelled.");
    assert!(start.elapsed() < Duration::from_secs(2));
    server.join().unwrap();
}
