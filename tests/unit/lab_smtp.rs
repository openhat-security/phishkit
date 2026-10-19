#[path = "support.rs"]
mod support;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use phishkit_core::mail::{self, smtp_security_mode, MailSettings, SmtpSecurityMode};
use support::isolate_storage;

/// Minimal cleartext SMTP sink that accepts one message and records the DATA body.
fn serve_plain_smtp() -> (
    u16,
    Arc<Mutex<String>>,
    Arc<AtomicUsize>,
    Arc<AtomicBool>,
    thread::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let body = Arc::new(Mutex::new(String::new()));
    let hits = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let body_c = body.clone();
    let hits_c = hits.clone();
    let stop_c = stop.clone();
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        while !stop_c.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    hits_c.fetch_add(1, Ordering::SeqCst);
                    let _ = stream.write_all(b"220 lab.phishkit SMTP\r\n");
                    let mut buf = Vec::new();
                    let mut tmp = [0u8; 1024];
                    let mut data_mode = false;
                    let mut data = Vec::new();
                    loop {
                        match stream.read(&mut tmp) {
                            Ok(0) => break,
                            Ok(n) => {
                                buf.extend_from_slice(&tmp[..n]);
                                while let Some(pos) = buf.windows(2).position(|w| w == b"\r\n") {
                                    let line = buf.drain(..pos + 2).collect::<Vec<u8>>();
                                    let line_s = String::from_utf8_lossy(&line).to_string();
                                    if data_mode {
                                        if line_s == ".\r\n" {
                                            *body_c.lock().unwrap() =
                                                String::from_utf8_lossy(&data).to_string();
                                            let _ = stream.write_all(b"250 OK\r\n");
                                            data_mode = false;
                                        } else {
                                            data.extend_from_slice(&line);
                                        }
                                        continue;
                                    }
                                    let upper = line_s.to_ascii_uppercase();
                                    if upper.starts_with("EHLO") || upper.starts_with("HELO") {
                                        let _ = stream.write_all(b"250 lab\r\n");
                                    } else if upper.starts_with("MAIL") || upper.starts_with("RCPT")
                                    {
                                        let _ = stream.write_all(b"250 OK\r\n");
                                    } else if upper.starts_with("DATA") {
                                        let _ = stream
                                            .write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n");
                                        data_mode = true;
                                    } else if upper.starts_with("QUIT") {
                                        let _ = stream.write_all(b"221 bye\r\n");
                                        return;
                                    } else {
                                        let _ = stream.write_all(b"250 OK\r\n");
                                    }
                                }
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(_) => break,
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });
    (port, body, hits, stop, handle)
}

#[test]
fn plaintext_lab_send_delivers_to_local_sink() {
    let (_tmp, _env) = isolate_storage();
    let (port, body, hits, stop, handle) = serve_plain_smtp();

    let settings = MailSettings {
        provider: "smtp".into(),
        host: "127.0.0.1".into(),
        port,
        username: String::new(),
        password: String::new(),
        from_email: "ops@lab.test".into(),
        from_name: "Ops".into(),
        use_starttls: false,
        api_key: String::new(),
        region: String::new(),
        domain: String::new(),
        smtp_security: "plain".into(),
        accept_invalid_certs: false,
    }
    .normalize();
    assert_eq!(smtp_security_mode(&settings), SmtpSecurityMode::Plain);

    let receipt = mail::send_message(
        &settings,
        "target@lab.test",
        "lab delivery test",
        "<p>hello from phishkit lab smtp</p>",
    )
    .expect("plain lab send must succeed");
    assert!(receipt.message.contains("smtp"), "{}", receipt.message);

    for _ in 0..50 {
        if hits.load(Ordering::SeqCst) > 0 && !body.lock().unwrap().is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        hits.load(Ordering::SeqCst) > 0,
        "sink must see a connection"
    );
    let captured = body.lock().unwrap().clone();
    assert!(
        captured
            .to_lowercase()
            .contains("hello from phishkit lab smtp")
            || captured.contains("lab delivery test"),
        "DATA body missing, got: {captured:?}"
    );

    stop.store(true, Ordering::SeqCst);
    let _ = handle.join();
}

#[test]
fn saved_smtp_security_plain_round_trips() {
    let (_tmp, _env) = isolate_storage();
    let settings = MailSettings {
        provider: "smtp".into(),
        host: "127.0.0.1".into(),
        port: 2525,
        username: String::new(),
        password: String::new(),
        from_email: "ops@lab.test".into(),
        from_name: "Ops".into(),
        use_starttls: true, // should be overridden by explicit plain
        api_key: String::new(),
        region: String::new(),
        domain: String::new(),
        smtp_security: "plain".into(),
        accept_invalid_certs: true,
    };
    mail::save_mail_settings(settings).expect("save");
    let loaded = mail::get_mail_settings().expect("load");
    assert_eq!(loaded.smtp_security, "plain");
    assert!(loaded.accept_invalid_certs);
    assert_eq!(smtp_security_mode(&loaded), SmtpSecurityMode::Plain);
}
