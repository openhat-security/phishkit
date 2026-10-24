#[path = "support.rs"]
mod support;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use phishkit_core::recon;
use support::isolate_storage;

/// Spin a tiny HTTP listener on 127.0.0.1:0 that counts accepted connections.
fn serve_counter() -> (
    u16,
    Arc<AtomicUsize>,
    Arc<AtomicBool>,
    thread::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let hits_c = hits.clone();
    let stop_c = stop.clone();
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        while !stop_c.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    hits_c.fetch_add(1, Ordering::SeqCst);
                    let mut buf = [0u8; 1024];
                    let _ = stream.read(&mut buf);
                    let body =
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
                    let _ = stream.write_all(body);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });
    (port, hits, stop, handle)
}

/// Audit repro: detect against 127.0.0.1 used to fetch. With the SSRF gate it
/// must refuse before any byte is written to the local listener.
#[test]
fn detect_refuses_loopback_and_never_hits_listener() {
    let (_tmp, _env) = isolate_storage();
    // Ensure the private allow env is not set from a prior test.
    std::env::remove_var("PHISHKIT_RECON_ALLOW_PRIVATE");
    std::env::remove_var("PHISHKIT_RECON_INSECURE_TLS");

    let (port, hits, stop, handle) = serve_counter();
    let target = format!("http://127.0.0.1:{port}/");

    let err = recon::detect_target(&target).expect_err("must refuse loopback");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("blocked") || msg.contains("refusing"),
        "expected SSRF refusal, got: {msg}"
    );

    // Give the accept loop a moment; hits must stay zero.
    thread::sleep(Duration::from_millis(50));
    assert_eq!(
        hits.load(Ordering::SeqCst),
        0,
        "SSRF gate must not open a TCP connection to the private listener"
    );

    stop.store(true, Ordering::SeqCst);
    let _ = handle.join();
}

/// Lab opt-in: with PHISHKIT_RECON_ALLOW_PRIVATE=1 the same target is allowed
/// through the gate and the listener sees at least one request.
#[test]
fn detect_reaches_loopback_when_private_opt_in() {
    let (_tmp, _env) = isolate_storage();
    std::env::set_var("PHISHKIT_RECON_ALLOW_PRIVATE", "1");
    std::env::remove_var("PHISHKIT_RECON_INSECURE_TLS");

    let (port, hits, stop, handle) = serve_counter();
    let target = format!("http://127.0.0.1:{port}/");

    // May succeed or fail on stack detection; either way the gate must not refuse.
    let result = recon::detect_target(&target);
    assert!(
        result.is_ok(),
        "with allow-private, detect should not SSRF-refuse, got: {result:?}"
    );
    // Wait for the accept loop to observe the connection(s).
    for _ in 0..50 {
        if hits.load(Ordering::SeqCst) > 0 {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        hits.load(Ordering::SeqCst) > 0,
        "opt-in path must actually fetch the private listener"
    );

    stop.store(true, Ordering::SeqCst);
    let _ = handle.join();
    std::env::remove_var("PHISHKIT_RECON_ALLOW_PRIVATE");
}
