//! The app's way to the hub, shared by every page: where it is, and the connection once made.

use std::sync::{Arc, Mutex};

use crate::api::User;
use crate::hub::{self, Error, Hub};

/// Where to find the hub: WALROOS_HUB and WALROOS_PROXY, as `walroos ui` takes --hub and --proxy.
#[derive(Clone)]
pub struct Target {
    pub hub: String,
    pub proxy: Option<String>,
}

impl Target {
    pub fn from_env() -> Target {
        let var = |k| std::env::var(k).ok().filter(|v: &String| !v.is_empty());
        Target {
            hub: var("WALROOS_HUB").unwrap_or_else(|| "walroos-hub".into()),
            proxy: var("WALROOS_PROXY"),
        }
    }
}

type Conn = Option<(Arc<Hub>, User)>;

#[derive(Clone)]
pub struct Link {
    target: Target,
    conn: Arc<Mutex<Conn>>,
}

impl Link {
    pub fn new(target: Target) -> Link {
        Link {
            target,
            conn: Arc::default(),
        }
    }

    /// Runs f against the hub, reaching it first if needed. Blocks: call it on a background thread.
    /// When the hub can't be reached, the connection is dropped so the next call finds the way
    /// again (tailscaled may have come up, or gone).
    pub fn call<T>(&self, f: impl FnOnce(&Hub) -> Result<T, Error>) -> Result<T, Error> {
        let current = self.conn.lock().unwrap().clone();
        let hub = match current {
            Some((h, _)) => h,
            None => {
                let (h, me) = hub::connect(&self.target.hub, self.target.proxy.as_deref())?;
                let h = Arc::new(h);
                *self.conn.lock().unwrap() = Some((h.clone(), me));
                h
            }
        };
        let result = f(&hub);
        if let Err(Error::Unreachable(_)) = result {
            *self.conn.lock().unwrap() = None;
        }
        result
    }

    /// Which hub, as whom and how, for the page header.
    pub fn describe(&self) -> String {
        match &*self.conn.lock().unwrap() {
            Some((h, me)) => match &h.proxy {
                Some(p) => format!("{} as {}, through tailscaled's proxy at {p}", h.url, me.login),
                None => format!("{} as {}", h.url, me.login),
            },
            None => format!("Reaching {}…", hub::hub_url(&self.target.hub)),
        }
    }
}
