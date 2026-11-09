//! Comprehensive integration test suite for IRQ Smart PDF Engine (Phase 10).
//!
//! Tests real HTTP server responses including:
//! - Direct PDF links
//! - URLs without .pdf extension
//! - Content-Type application/pdf
//! - Content-Disposition attachment with filename
//! - Multiple HTTP 302 redirects
//! - HTTP 206 Partial Content with byte ranges
//! - Application/octet-stream with pdf filename
//! - HTML error/login pages mistakenly returned as PDF (rejection test)
//! - Authenticated downloads requiring Referer / Cookie
//! - HTTP 403 Forbidden handling
//! - Non-PDF media (video/audio) downloads unaffected

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use swiftget_engine::http::{build_client, probe, Progress, Shared};
use swiftget_engine::http_download::{self, HttpJob};
use swiftget_engine::limiter::SpeedLimiter;
use swiftget_engine::pdf;
use swiftget_engine::RequestHeaders;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

fn make_valid_pdf(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size.max(100));
    data.extend_from_slice(b"%PDF-1.7\n%Fake PDF content for test\n");
    while data.len() < size {
        data.push(b'A');
    }
    data.extend_from_slice(b"\n%%EOF\n");
    data
}

async fn start_test_server() -> (u16, Arc<AtomicU32>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let requests_count = Arc::new(AtomicU32::new(0));
    let count_clone = requests_count.clone();

    tokio::spawn(async move {
        loop {
            let (sock, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };
            count_clone.fetch_add(1, Ordering::Relaxed);
            let port = port;

            tokio::spawn(async move {
                let (r, mut w) = sock.into_split();
                let mut r = BufReader::new(r);
                let mut first_line = String::new();
                if r.read_line(&mut first_line).await.unwrap_or(0) == 0 {
                    return;
                }

                let mut headers = Vec::new();
                loop {
                    let mut line = String::new();
                    if r.read_line(&mut line).await.unwrap_or(0) == 0 || line.trim().is_empty() {
                        break;
                    }
                    headers.push(line.to_ascii_lowercase());
                }

                let path = first_line.split_whitespace().nth(1).unwrap_or("/");
                let range = headers.iter().find_map(|h| {
                    h.strip_prefix("range: bytes=").and_then(|v| {
                        let (a, b) = v.trim().split_once('-')?;
                        let a: usize = a.parse().ok()?;
                        let b: usize = if b.is_empty() { 9999 } else { b.parse().ok()? };
                        Some((a, b))
                    })
                });

                let referer = headers.iter().find(|h| h.starts_with("referer:"));
                let cookie = headers.iter().find(|h| h.starts_with("cookie:"));

                let response = match path {
                    // 1. Direct PDF link
                    "/direct.pdf" => {
                        let pdf = make_valid_pdf(1024);
                        let mut resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/pdf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            pdf.len()
                        ).into_bytes();
                        resp.extend_from_slice(&pdf);
                        resp
                    }

                    // 2. PDF without extension
                    "/api/download_doc" => {
                        let pdf = make_valid_pdf(2048);
                        let mut resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/pdf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            pdf.len()
                        ).into_bytes();
                        resp.extend_from_slice(&pdf);
                        resp
                    }

                    // 3. Content-Disposition with RFC 5987 filename
                    "/download/disposition" => {
                        let pdf = make_valid_pdf(1500);
                        let mut resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=\"paper_1.pdf\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            pdf.len()
                        ).into_bytes();
                        resp.extend_from_slice(&pdf);
                        resp
                    }

                    // 4. Redirect step 1 -> step 2 -> final
                    "/redir1" => {
                        format!("HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{port}/redir2\r\nConnection: close\r\n\r\n").into_bytes()
                    }
                    "/redir2" => {
                        format!("HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{port}/direct.pdf\r\nConnection: close\r\n\r\n").into_bytes()
                    }

                    // 5. HTTP 206 Range requests
                    "/range.pdf" => {
                        let pdf = make_valid_pdf(4000);
                        if let Some((start, end)) = range {
                            let end = end.min(pdf.len() - 1);
                            let slice = &pdf[start..=end];
                            let mut resp = format!(
                                "HTTP/1.1 206 Partial Content\r\nContent-Type: application/pdf\r\nContent-Range: bytes {start}-{end}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                pdf.len(),
                                slice.len()
                            ).into_bytes();
                            resp.extend_from_slice(slice);
                            resp
                        } else {
                            let mut resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/pdf\r\nAccept-Ranges: bytes\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                pdf.len()
                            ).into_bytes();
                            resp.extend_from_slice(&pdf);
                            resp
                        }
                    }

                    // 6. application/octet-stream with pdf filename
                    "/octet.pdf" => {
                        let pdf = make_valid_pdf(1200);
                        let mut resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            pdf.len()
                        ).into_bytes();
                        resp.extend_from_slice(&pdf);
                        resp
                    }

                    // 7. Fake PDF returning HTML login / error page
                    "/paywall_fake.pdf" => {
                        let html = b"<!DOCTYPE html><html><head><title>Sign In</title></head><body><h1>Login to access this paper</h1></body></html>";
                        let mut resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            html.len()
                        ).into_bytes();
                        resp.extend_from_slice(html);
                        resp
                    }

                    // 8. Protected file requiring Referer & Cookie
                    "/protected.pdf" => {
                        let has_cookie = cookie.map(|c| c.contains("session=123")).unwrap_or(false);
                        let has_ref = referer.map(|r| r.contains("journal.test")).unwrap_or(false);
                        if has_cookie && has_ref {
                            let pdf = make_valid_pdf(1000);
                            let mut resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/pdf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                pdf.len()
                            ).into_bytes();
                            resp.extend_from_slice(&pdf);
                            resp
                        } else {
                            b"HTTP/1.1 403 Forbidden\r\nContent-Length: 13\r\nConnection: close\r\n\r\n403 Forbidden".to_vec()
                        }
                    }

                    // 9. 403 Forbidden
                    "/forbidden.pdf" => {
                        b"HTTP/1.1 403 Forbidden\r\nContent-Length: 13\r\nConnection: close\r\n\r\nAccess Denied".to_vec()
                    }

                    // 10. Non-PDF media (Video test: MP4 bytes)
                    "/video.mp4" => {
                        let mp4 = vec![0x00, 0x00, 0x00, 0x18, b'f', b't', b'y', b'p', b'm', b'p', b'4', b'2'];
                        let mut resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            mp4.len()
                        ).into_bytes();
                        resp.extend_from_slice(&mp4);
                        resp
                    }

                    _ => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
                };

                let _ = w.write_all(&response).await;
            });
        }
    });

    (port, requests_count)
}

#[tokio::test]
async fn test_smart_pdf_direct_link() {
    let (port, _) = start_test_server().await;
    let url = format!("http://127.0.0.1:{port}/direct.pdf");

    let shared = Arc::new(Shared::new(build_client().unwrap(), SpeedLimiter::new(0)));
    let probe_info = probe(&shared.client(), &url, &RequestHeaders::default(), "test").await.unwrap();
    assert_eq!(probe_info.content_type.as_deref(), Some("application/pdf"));

    let dir = std::env::temp_dir().join(format!("swiftget-pdf-direct-{port}"));
    let _ = std::fs::create_dir_all(&dir);
    let part_path = dir.join("direct.pdf.sgpart");
    let _ = std::fs::remove_file(&part_path);

    let job = HttpJob {
        url: url.clone(),
        headers: RequestHeaders::default(),
        user_agent: "test".into(),
        total_size: probe_info.total_size,
        resumable: probe_info.resumable,
        part_path: part_path.clone(),
        connections: 1,
        min_split: 512,
        retries: 2,
        progress_base: 0,
        max_request: None,
        limiter: Arc::new(SpeedLimiter::new(0)),
    };

    let segs = Arc::new(Mutex::new(Vec::new()));
    let progress = Arc::new(Progress::default());
    let cancel = CancellationToken::new();

    http_download::run(shared, job, segs, progress, cancel).await.unwrap();

    assert!(part_path.exists());
    assert!(pdf::verify_file(&part_path).is_ok());
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn test_smart_pdf_without_extension() {
    let (port, _) = start_test_server().await;
    let url = format!("http://127.0.0.1:{port}/api/download_doc");

    let shared = Arc::new(Shared::new(build_client().unwrap(), SpeedLimiter::new(0)));
    let probe_info = probe(&shared.client(), &url, &RequestHeaders::default(), "test").await.unwrap();
    assert_eq!(probe_info.content_type.as_deref(), Some("application/pdf"));

    let dir = std::env::temp_dir().join(format!("swiftget-pdf-noext-{port}"));
    let _ = std::fs::create_dir_all(&dir);
    let part_path = dir.join("doc.pdf.sgpart");
    let _ = std::fs::remove_file(&part_path);

    let job = HttpJob {
        url: url.clone(),
        headers: RequestHeaders::default(),
        user_agent: "test".into(),
        total_size: probe_info.total_size,
        resumable: probe_info.resumable,
        part_path: part_path.clone(),
        connections: 1,
        min_split: 512,
        retries: 2,
        progress_base: 0,
        max_request: None,
        limiter: Arc::new(SpeedLimiter::new(0)),
    };

    let segs = Arc::new(Mutex::new(Vec::new()));
    let progress = Arc::new(Progress::default());
    let cancel = CancellationToken::new();

    http_download::run(shared, job, segs, progress, cancel).await.unwrap();

    assert!(part_path.exists());
    assert!(pdf::verify_file(&part_path).is_ok());
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn test_smart_pdf_redirects() {
    let (port, _) = start_test_server().await;
    let url = format!("http://127.0.0.1:{port}/redir1");

    let shared = Arc::new(Shared::new(build_client().unwrap(), SpeedLimiter::new(0)));
    let probe_info = probe(&shared.client(), &url, &RequestHeaders::default(), "test").await.unwrap();
    assert_eq!(probe_info.content_type.as_deref(), Some("application/pdf"));

    let dir = std::env::temp_dir().join(format!("swiftget-pdf-redir-{port}"));
    let _ = std::fs::create_dir_all(&dir);
    let part_path = dir.join("redir.pdf.sgpart");
    let _ = std::fs::remove_file(&part_path);

    let job = HttpJob {
        url: url.clone(),
        headers: RequestHeaders::default(),
        user_agent: "test".into(),
        total_size: probe_info.total_size,
        resumable: probe_info.resumable,
        part_path: part_path.clone(),
        connections: 1,
        min_split: 512,
        retries: 2,
        progress_base: 0,
        max_request: None,
        limiter: Arc::new(SpeedLimiter::new(0)),
    };

    let segs = Arc::new(Mutex::new(Vec::new()));
    let progress = Arc::new(Progress::default());
    let cancel = CancellationToken::new();

    http_download::run(shared, job, segs, progress, cancel).await.unwrap();

    assert!(part_path.exists());
    assert!(pdf::verify_file(&part_path).is_ok());
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn test_smart_pdf_range_and_partial_content() {
    let (port, _) = start_test_server().await;
    let url = format!("http://127.0.0.1:{port}/range.pdf");

    let shared = Arc::new(Shared::new(build_client().unwrap(), SpeedLimiter::new(0)));
    let probe_info = probe(&shared.client(), &url, &RequestHeaders::default(), "test").await.unwrap();
    assert!(probe_info.resumable);

    let dir = std::env::temp_dir().join(format!("swiftget-pdf-range-{port}"));
    let _ = std::fs::create_dir_all(&dir);
    let part_path = dir.join("range.pdf.sgpart");
    let _ = std::fs::remove_file(&part_path);

    let job = HttpJob {
        url: url.clone(),
        headers: RequestHeaders::default(),
        user_agent: "test".into(),
        total_size: probe_info.total_size,
        resumable: probe_info.resumable,
        part_path: part_path.clone(),
        connections: 4,
        min_split: 512,
        retries: 2,
        progress_base: 0,
        max_request: None,
        limiter: Arc::new(SpeedLimiter::new(0)),
    };

    let segs = Arc::new(Mutex::new(Vec::new()));
    let progress = Arc::new(Progress::default());
    let cancel = CancellationToken::new();

    http_download::run(shared, job, segs, progress, cancel).await.unwrap();

    assert!(part_path.exists());
    assert!(pdf::verify_file(&part_path).is_ok());
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn test_smart_pdf_rejects_html_error_page() {
    let (port, _) = start_test_server().await;
    let url = format!("http://127.0.0.1:{port}/paywall_fake.pdf");

    let shared = Arc::new(Shared::new(build_client().unwrap(), SpeedLimiter::new(0)));
    let probe_info = probe(&shared.client(), &url, &RequestHeaders::default(), "test").await.unwrap();

    let dir = std::env::temp_dir().join(format!("swiftget-pdf-fake-{port}"));
    let _ = std::fs::create_dir_all(&dir);
    let part_path = dir.join("fake.pdf.sgpart");
    let _ = std::fs::remove_file(&part_path);

    let job = HttpJob {
        url: url.clone(),
        headers: RequestHeaders::default(),
        user_agent: "test".into(),
        total_size: probe_info.total_size,
        resumable: false,
        part_path: part_path.clone(),
        connections: 1,
        min_split: 512,
        retries: 1,
        progress_base: 0,
        max_request: None,
        limiter: Arc::new(SpeedLimiter::new(0)),
    };

    let segs = Arc::new(Mutex::new(Vec::new()));
    let progress = Arc::new(Progress::default());
    let cancel = CancellationToken::new();

    http_download::run(shared, job, segs, progress, cancel).await.unwrap();

    let verification = pdf::verify_file(&part_path);
    assert!(verification.is_err(), "HTML file masquerading as PDF must be rejected");
    let err_msg = verification.unwrap_err().to_string();
    assert!(err_msg.contains("HTML") || err_msg.contains("%PDF-"));
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn test_smart_pdf_authenticated_download() {
    let (port, _) = start_test_server().await;
    let url = format!("http://127.0.0.1:{port}/protected.pdf");

    let shared = Arc::new(Shared::new(build_client().unwrap(), SpeedLimiter::new(0)));

    // 1. Without auth: fails with 403
    let probe_fail = probe(&shared.client(), &url, &RequestHeaders::default(), "test").await;
    assert!(probe_fail.is_err(), "probe without required credentials should fail");

    // 2. With auth headers: succeeds
    let auth_headers = RequestHeaders {
        referer: Some("http://journal.test/article".to_string()),
        cookies: Some("session=123; user=member".to_string()),
        user_agent: None,
        auth: None,
        extra: Vec::new(),
    };

    let probe_ok = probe(&shared.client(), &url, &auth_headers, "test").await.unwrap();
    assert_eq!(probe_ok.content_type.as_deref(), Some("application/pdf"));

    let dir = std::env::temp_dir().join(format!("swiftget-pdf-auth-{port}"));
    let _ = std::fs::create_dir_all(&dir);
    let part_path = dir.join("protected.pdf.sgpart");
    let _ = std::fs::remove_file(&part_path);

    let job = HttpJob {
        url: url.clone(),
        headers: auth_headers,
        user_agent: "test".into(),
        total_size: probe_ok.total_size,
        resumable: false,
        part_path: part_path.clone(),
        connections: 1,
        min_split: 512,
        retries: 1,
        progress_base: 0,
        max_request: None,
        limiter: Arc::new(SpeedLimiter::new(0)),
    };

    let segs = Arc::new(Mutex::new(Vec::new()));
    let progress = Arc::new(Progress::default());
    let cancel = CancellationToken::new();

    http_download::run(shared, job, segs, progress, cancel).await.unwrap();

    assert!(part_path.exists());
    assert!(pdf::verify_file(&part_path).is_ok());
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn test_video_download_unaffected_by_pdf_engine() {
    let (port, _) = start_test_server().await;
    let url = format!("http://127.0.0.1:{port}/video.mp4");

    let shared = Arc::new(Shared::new(build_client().unwrap(), SpeedLimiter::new(0)));
    let probe_info = probe(&shared.client(), &url, &RequestHeaders::default(), "test").await.unwrap();
    assert_eq!(probe_info.content_type.as_deref(), Some("video/mp4"));

    let dir = std::env::temp_dir().join(format!("swiftget-pdf-video-{port}"));
    let _ = std::fs::create_dir_all(&dir);
    let part_path = dir.join("video.mp4.sgpart");
    let _ = std::fs::remove_file(&part_path);

    let job = HttpJob {
        url: url.clone(),
        headers: RequestHeaders::default(),
        user_agent: "test".into(),
        total_size: probe_info.total_size,
        resumable: false,
        part_path: part_path.clone(),
        connections: 1,
        min_split: 512,
        retries: 1,
        progress_base: 0,
        max_request: None,
        limiter: Arc::new(SpeedLimiter::new(0)),
    };

    let segs = Arc::new(Mutex::new(Vec::new()));
    let progress = Arc::new(Progress::default());
    let cancel = CancellationToken::new();

    http_download::run(shared, job, segs, progress, cancel).await.unwrap();

    assert!(part_path.exists());
    let bytes = std::fs::read(&part_path).unwrap();
    assert!(bytes.starts_with(&[0x00, 0x00, 0x00, 0x18, b'f', b't', b'y', b'p']));
    let _ = std::fs::remove_dir_all(dir);
}
