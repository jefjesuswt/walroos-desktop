//! How this machine reaches the hub and what it asks it. The hub knows you by your tailnet
//! identity (Tailscale's WhoIs on the connection), so there is no login: the request only has
//! to come from this machine on the tailnet. Directly when tailscaled routes to it, or through
//! tailscaled's HTTP proxy when it runs in userspace mode (as next to another VPN). Same logic
//! as `cmd/walroos/hubconn` in the walroos repo.

use std::fmt;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde::de::{DeserializeOwned, IgnoredAny};
use serde_json::json;
use ureq::{Agent, Proxy};

use crate::api::{Deployment, Node, Project, User};

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
    /// A hub answered and doesn't know you: it's a hub, your login just isn't on it.
    Stranger(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unreachable(m) | Error::Answer(m) | Error::Stranger(m) => f.write_str(m),
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
            Err(Error::Unreachable(_)) => {}
            Err(answer) => return Err(answer),
        }
    }
    Err(Error::Unreachable(format!(
        "reach {url}: {direct_err} (is the hub's name right in Settings, and this machine on the tailnet? With userspace tailscaled, its proxy must be up: WALROOS_PROXY={DEFAULT_PROXY})"
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

    /// Deploys sha, or the branch head when it's empty.
    pub fn deploy(&self, app: &str, sha: &str) -> Result<Deployment, Error> {
        self.post(&format!("/api/apps/{app}/deploy"), json!({ "sha": sha }))
    }

    /// action is stop, start or restart.
    pub fn power(&self, app: &str, action: &str) -> Result<(), Error> {
        self.post::<IgnoredAny>(&format!("/api/apps/{app}/power"), json!({ "action": action }))
            .map(|_| ())
    }

    pub fn deployments(&self, app: &str) -> Result<Vec<Deployment>, Error> {
        self.get(&format!("/api/apps/{app}/deployments"))
    }

    /// Every deploy of every app, newest first.
    pub fn all_deployments(&self) -> Result<Vec<Deployment>, Error> {
        self.get("/api/deployments")
    }

    /// One deployment, with its build log.
    pub fn deployment(&self, app: &str, id: &str) -> Result<Deployment, Error> {
        self.get(&format!("/api/apps/{app}/deployments/{id}"))
    }

    /// The last lines the app's running release wrote.
    pub fn app_logs(&self, app: &str, lines: u32) -> Result<String, Error> {
        let path = format!("/api/apps/{app}/logs?lines={lines}");
        let resp = self.agent.get(&format!("{}{path}", self.url)).call();
        self.answer(resp, &path, false)
    }

    pub fn rollback(&self, app: &str, deployment: &str) -> Result<Deployment, Error> {
        self.post(
            &format!("/api/apps/{app}/deployments/{deployment}/rollback"),
            json!({}),
        )
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let resp = self
            .agent
            .get(&format!("{}{path}", self.url))
            .header("Accept", "application/json")
            .call();
        self.json(self.answer(resp, path, false)?, path)
    }

    fn post<T: DeserializeOwned>(&self, path: &str, body: serde_json::Value) -> Result<T, Error> {
        let url = format!("{}{path}", self.url);
        let resp = self
            .agent
            .post(&url)
            .header("Accept", "application/json")
            .send_json(body);
        self.json(self.answer(resp, path, true)?, path)
    }

    fn put<T: DeserializeOwned>(&self, path: &str, body: serde_json::Value) -> Result<T, Error> {
        let url = format!("{}{path}", self.url);
        let resp = self
            .agent
            .put(&url)
            .header("Accept", "application/json")
            .send_json(body);
        self.json(self.answer(resp, path, true)?, path)
    }

    fn delete(&self, path: &str) -> Result<(), Error> {
        let url = format!("{}{path}", self.url);
        let resp = self
            .agent
            .delete(&url)
            .header("Accept", "application/json")
            .call();
        self.answer(resp, path, true).map(|_| ())
    }

    fn json<T: DeserializeOwned>(&self, body: String, path: &str) -> Result<T, Error> {
        let body = if body.is_empty() { "null" } else { &body };
        serde_json::from_str(body)
            .map_err(|e| Error::Answer(format!("{} answered {path} with something else: {e}", self.url)))
    }

    /// The body of a 2xx answer, or what went wrong.
    fn answer(
        &self,
        resp: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
        path: &str,
        change: bool,
    ) -> Result<String, Error> {
        let mut resp = resp.map_err(|e| Error::Unreachable(e.to_string()))?;
        let status = resp.status().as_u16();
        let body = resp.body_mut().read_to_string().unwrap_or_default();
        if (200..300).contains(&status) {
            return Ok(body);
        }
        let said = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v["error"].as_str().map(str::to_string))
            .unwrap_or_default();
        Err(match status {
            502 if self.proxy.is_some() => Error::Unreachable(format!(
                "nothing answered at {} on the tailnet (502 from the proxy)",
                self.url
            )),
            // A change your role can't make is a 403 too: say what the hub said.
            401 | 403 if !change && !said.is_empty() => {
                Error::Stranger(format!("the hub doesn't know you: {said}"))
            }
            401 | 403 if !change => Error::Answer(format!("{} answered {status} to {path}", self.url)),
            _ if !said.is_empty() => Error::Answer(said),
            _ => Error::Answer(format!("{} answered {status} to {path}", self.url)),
        })
    }
}

mod apps;
mod settings;
#[cfg(test)]
mod tests;
