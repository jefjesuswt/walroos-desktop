use super::*;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

type Seen = Arc<Mutex<Vec<String>>>;

/// A fake hub (or proxy in front of one) on 127.0.0.1: answers `status` with `body` to
/// every request. A CONNECT gets a 200 and the request after it on the same connection.
/// Returns its address and the request lines it saw.
fn fake(status: u16, body: &'static str) -> (String, Seen) {
    let ln = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = ln.local_addr().unwrap().to_string();
    let seen: Seen = Arc::default();
    let log = seen.clone();
    thread::spawn(move || {
        for conn in ln.incoming().flatten() {
            let log = log.clone();
            thread::spawn(move || answer(conn, status, body, log));
        }
    });
    (addr, seen)
}

fn answer(mut conn: TcpStream, status: u16, body: &str, seen: Seen) {
    let mut r = BufReader::new(conn.try_clone().unwrap());
    let mut line = read_head(&mut r);
    if line.is_empty() {
        return; // a probe that only checked something listens
    }
    if line.starts_with("CONNECT") {
        seen.lock().unwrap().push(line);
        conn.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .unwrap();
        line = read_head(&mut r);
    }
    seen.lock().unwrap().push(line);
    let resp = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    conn.write_all(resp.as_bytes()).unwrap();
}

/// Reads a request head and returns its first line.
fn read_head(r: &mut impl BufRead) -> String {
    let mut first = String::new();
    if r.read_line(&mut first).unwrap_or(0) == 0 {
        return first;
    }
    let mut line = String::new();
    while r.read_line(&mut line).unwrap() > 2 {
        line.clear();
    }
    first.trim_end().to_string()
}

/// An address where nothing listens.
fn closed() -> String {
    let ln = TcpListener::bind("127.0.0.1:0").unwrap();
    ln.local_addr().unwrap().to_string()
}

const ME: &str = r#"{"login":"dev@example.com","role":"owner"}"#;

#[test]
fn reaches_the_hub_directly() {
    let (hub, seen) = fake(200, ME);
    let (h, me) = connect_with(&hub, None, &closed()).unwrap();
    assert_eq!(me.login, "dev@example.com");
    assert_eq!(h.proxy, None);
    assert_eq!(seen.lock().unwrap()[0], "GET /api/me HTTP/1.1");
}

#[test]
fn falls_back_to_tailscaleds_proxy() {
    let hub = closed();
    let (proxy, seen) = fake(200, ME);
    let (h, me) = connect_with(&hub, None, &proxy).unwrap();
    assert_eq!(me.login, "dev@example.com");
    assert_eq!(h.proxy.as_deref(), Some(proxy.as_str()));
    let seen = seen.lock().unwrap();
    assert_eq!(seen[0], format!("CONNECT {hub} HTTP/1.1"));
    assert_eq!(seen[1], "GET /api/me HTTP/1.1");
}

#[test]
fn goes_only_through_the_proxy_given() {
    let (proxy, seen) = fake(200, ME);
    let (h, _) = connect_with("walroos-hub", Some(&format!("http://{proxy}")), &closed()).unwrap();
    assert_eq!(h.url, "http://walroos-hub");
    assert_eq!(seen.lock().unwrap()[0], "CONNECT walroos-hub:80 HTTP/1.1");
}

#[test]
fn a_hub_that_says_no_is_not_retried() {
    let (hub, _) = fake(403, r#"{"error":"not a member"}"#);
    let (proxy, seen) = fake(200, ME);
    let err = connect_with(&hub, None, &proxy).err().unwrap();
    assert_eq!(
        err,
        Error::Stranger("the hub doesn't know you: not a member".into())
    );
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn says_what_to_check_when_nothing_answers() {
    let Err(Error::Unreachable(m)) = connect_with(&closed(), None, &closed()) else {
        panic!("expected Unreachable");
    };
    assert!(m.contains("WALROOS_PROXY=localhost:1055"), "{m}");
}

#[test]
fn a_change_is_done_when_the_hub_accepts_it() {
    let (hub, seen) = fake(202, r#"{"id":"d1","state":"queued"}"#);
    let h = Hub::new(&hub_url(&hub), None, TIMEOUT).unwrap();
    h.deploy("web").unwrap();
    assert_eq!(seen.lock().unwrap()[0], "POST /api/apps/web/deploy HTTP/1.1");
    let (hub, _) = fake(204, "");
    Hub::new(&hub_url(&hub), None, TIMEOUT)
        .unwrap()
        .power("web", "stop")
        .unwrap();
}

#[test]
fn a_change_your_role_cant_make_says_so() {
    let (hub, _) = fake(
        403,
        r#"{"error":"your role can't do that: developer users can't manage apps"}"#,
    );
    let h = Hub::new(&hub_url(&hub), None, TIMEOUT).unwrap();
    let err = h.rollback("web", "d1").err().unwrap();
    assert_eq!(
        err,
        Error::Answer("your role can't do that: developer users can't manage apps".into())
    );
}

#[test]
fn normalizes_hub_and_proxy() {
    assert_eq!(hub_url("walroos-hub/"), "http://walroos-hub");
    assert_eq!(hub_url("https://hub.example.com"), "https://hub.example.com");
    assert_eq!(normalize_proxy("socks5://localhost:1055"), "localhost:1055");
    assert_eq!(normalize_proxy("localhost"), "localhost:1055");
}
