//! How this machine reaches the hub and what it asks it. The hub knows you by your tailnet
//! identity (Tailscale's WhoIs on the connection), so there is no login: the request only has
//! to come from this machine on the tailnet. Directly when tailscaled routes to it, or through
//! tailscaled's HTTP proxy when it runs in userspace mode (as next to another VPN). Same logic
//! as `cmd/walroos/hubconn` in the walroos repo.

use std::fmt;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde::de::DeserializeOwned;
use ureq::{Agent, Proxy};

use crate::api::{Node, Project, User};

/// Where tailscaled in userspace mode serves its HTTP/SOCKS5 proxy (the port its docs use).
pub const DEFAULT_PROXY: &str = "localhost:1055";

const DIRECT_TRY: Duration = Duration::from_secs(4);
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    /// Nothing answered: no route to the tailnet, the hub is down, or the proxy isn't up.
    Unreachable(String),
    /// The hub answered but said no. Trying another way won't help.
    Answer(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unreachable(m) | Error::Answer(m) => f.write_str(m),
        }
    }
}

/// A hub this app has reached, and how.
pub struct Hub {
    pub url: String,
    /// The proxy requests go through; None for direct.
    pub proxy: Option<String>,
    agent: Agent,
}

/// `hub` with http:// when it has no scheme, without a trailing slash.
pub fn hub_url(hub: &str) -> String {
    let hub = if hub.contains("://") {
        hub.to_string()
    } else {
        format!("http://{hub}")
    };
    hub.trim_end_matches('/').to_string()
}

/// Accepts localhost:1055, http://localhost:1055 or socks5://localhost:1055 and returns
/// host:port (tailscaled serves HTTP and SOCKS5 on the same port; this app uses HTTP CONNECT).
pub fn normalize_proxy(p: &str) -> String {
    let p = p
        .trim_start_matches("http://")
        .trim_start_matches("socks5://")
        .trim_end_matches('/');
    if p.rsplit_once(':')
        .is_some_and(|(_, port)| port.parse::<u16>().is_ok())
    {
        p.to_string()
    } else {
        format!("{p}:1055")
    }
}

/// Reaches the hub and asks who you are. With `proxy` it only goes through it. Without, it
/// tries the tailnet directly, briefly; if that fails and tailscaled's proxy answers on
/// DEFAULT_PROXY, it goes through it.
pub fn connect(hub: &str, proxy: Option<&str>) -> Result<(Hub, User), Error> {
    connect_with(hub, proxy, DEFAULT_PROXY)
}

fn connect_with(hub: &str, proxy: Option<&str>, default_proxy: &str) -> Result<(Hub, User), Error> {
    let url = hub_url(hub);
    if let Some(p) = proxy {
        let h = Hub::new(&url, Some(&normalize_proxy(p)), TIMEOUT)?;
        let me = h.me().map_err(|e| match e {
            Error::Unreachable(m) => {
                Error::Unreachable(format!("reach {url} through {}: {m}", normalize_proxy(p)))
            }
            answer => answer,
        })?;
        return Ok((h, me));
    }
    let direct_err = match Hub::new(&url, None, DIRECT_TRY)?.me() {
        Ok(me) => return Ok((Hub::new(&url, None, TIMEOUT)?, me)),
        Err(Error::Unreachable(m)) => m,
        Err(answer) => return Err(answer),
    };
    if listening(default_proxy) {
        let h = Hub::new(&url, Some(default_proxy), TIMEOUT)?;
        match h.me() {
            Ok(me) => return Ok((h, me)),
            Err(Error::Answer(m)) => return Err(Error::Answer(m)),
            Err(Error::Unreachable(_)) => {}
        }
    }
    Err(Error::Unreachable(format!(
        "reach {url}: {direct_err} (is this machine on the tailnet? With userspace tailscaled, its proxy must be up: WALROOS_PROXY={DEFAULT_PROXY})"
    )))
}

/// Something accepts TCP connections at addr (host:port).
fn listening(addr: &str) -> bool {
    let Ok(mut addrs) = addr.to_socket_addrs() else {
        return false;
    };
    addrs.any(|a| TcpStream::connect_timeout(&a, Duration::from_secs(1)).is_ok())
}

impl Hub {
    fn new(url: &str, proxy: Option<&str>, timeout: Duration) -> Result<Hub, Error> {
        let proxy_cfg = match proxy {
            Some(p) => Some(
                Proxy::new(&format!("http://{p}")).map_err(|e| Error::Answer(format!("proxy {p}: {e}")))?,
            ),
            None => None, // and not HTTP_PROXY from the environment: direct means direct
        };
        let agent = Agent::config_builder()
            .proxy(proxy_cfg)
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .build()
            .new_agent();
        Ok(Hub {
            url: url.to_string(),
            proxy: proxy.map(str::to_string),
            agent,
        })
    }

    pub fn me(&self) -> Result<User, Error> {
        self.get("/api/me")
    }

    pub fn nodes(&self) -> Result<Vec<Node>, Error> {
        self.get("/api/nodes")
    }

    pub fn projects(&self) -> Result<Vec<Project>, Error> {
        self.get("/api/projects")
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let mut resp = self
            .agent
            .get(&format!("{}{path}", self.url))
            .header("Accept", "application/json")
            .call()
            .map_err(|e| Error::Unreachable(e.to_string()))?;
        let status = resp.status().as_u16();
        if status == 200 {
            return resp.body_mut().read_json().map_err(|e| {
                Error::Answer(format!("{} answered {path} with something else: {e}", self.url))
            });
        }
        let said = resp
            .body_mut()
            .read_json::<serde_json::Value>()
            .ok()
            .and_then(|v| v["error"].as_str().map(str::to_string))
            .unwrap_or_default();
        Err(match status {
            502 if self.proxy.is_some() => Error::Unreachable(format!(
                "nothing answered at {} on the tailnet (502 from the proxy)",
                self.url
            )),
            401 | 403 => Error::Answer(format!("the hub doesn't know you: {said}")),
            _ => Error::Answer(format!("{} answered {status} to {path}: {said}", self.url)),
        })
    }
}

#[cfg(test)]
mod tests {
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
            Error::Answer("the hub doesn't know you: not a member".into())
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
    fn normalizes_hub_and_proxy() {
        assert_eq!(hub_url("walroos-hub/"), "http://walroos-hub");
        assert_eq!(hub_url("https://hub.example.com"), "https://hub.example.com");
        assert_eq!(normalize_proxy("socks5://localhost:1055"), "localhost:1055");
        assert_eq!(normalize_proxy("localhost"), "localhost:1055");
    }
}
