//! The app's way to the hub, shared by every page: where it is, and the connection once made.

use std::sync::{Arc, Mutex};

use crate::api::User;
use crate::hub::{self, Error, Hub};

/// Where to find the hub, as `walroos ui` takes --hub and --proxy (see config for where it comes from).
#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub hub: String,
    pub proxy: Option<String>,
}

type Conn = Option<(Arc<Hub>, User)>;

#[derive(Clone)]
pub struct Link {
    target: Arc<Mutex<Target>>,
    conn: Arc<Mutex<Conn>>,
}

impl Link {
    pub fn new(target: Target) -> Link {
        Link {
            target: Arc::new(Mutex::new(target)),
            conn: Arc::default(),
        }
    }

    pub fn target(&self) -> Target {
        self.target.lock().unwrap().clone()
    }

    /// Points every page at another hub, already reached (Settings checks it before saving).
    pub fn switch(&self, target: Target, hub: Hub, me: User) {
        *self.target.lock().unwrap() = target;
        *self.conn.lock().unwrap() = Some((Arc::new(hub), me));
    }

    /// Runs f against the hub, reaching it first if needed. Blocks: call it on a background thread.
    /// When the hub can't be reached, the connection is dropped so the next call finds the way
    /// again (tailscaled may have come up, or gone).
    pub fn call<T>(&self, f: impl FnOnce(&Hub) -> Result<T, Error>) -> Result<T, Error> {
        let current = self.conn.lock().unwrap().clone();
        let hub = match current {
            Some((h, _)) => h,
            None => {
                let t = self.target();
                let (h, me) = hub::connect(&t.hub, t.proxy.as_deref())?;
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

    /// Who the hub says you are, once reached.
    pub fn me(&self) -> Option<User> {
        self.conn.lock().unwrap().as_ref().map(|(_, me)| me.clone())
    }

    /// Which hub, as whom and how, for the page header.
    pub fn describe(&self) -> String {
        match &*self.conn.lock().unwrap() {
            Some((h, me)) => match &h.proxy {
                Some(p) => format!("{} as {}, through tailscaled's proxy at {p}", h.url, me.login),
                None => format!("{} as {}", h.url, me.login),
            },
            None => format!("Reaching {}…", hub::hub_url(&self.target().hub)),
        }
    }
}
