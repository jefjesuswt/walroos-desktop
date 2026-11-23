//! What the Settings tab asks the hub: its users, the Cloudflare and GitHub tokens it keeps, and
//! your SSH keys. Paths as in `web/src/api.ts`.

use serde::de::IgnoredAny;
use serde_json::json;

use super::{Error, Hub};
use crate::api::{Cloudflare, GitHub, Member, SshKey};

impl Hub {
    pub fn users(&self) -> Result<Vec<Member>, Error> {
        self.get("/api/users")
    }

    pub fn add_user(&self, login: &str, role: &str) -> Result<(), Error> {
        self.post::<IgnoredAny>("/api/users", json!({ "login": login, "role": role }))
            .map(|_| ())
    }

    pub fn set_role(&self, login: &str, role: &str) -> Result<(), Error> {
        self.put::<IgnoredAny>(&format!("/api/users/{login}"), json!({ "role": role }))
            .map(|_| ())
    }

    pub fn remove_user(&self, login: &str) -> Result<(), Error> {
        self.delete(&format!("/api/users/{login}"))
    }

    pub fn cloudflare(&self) -> Result<Cloudflare, Error> {
        self.get("/api/settings/cloudflare")
    }

    /// The hub checks the token with Cloudflare before it keeps it.
    pub fn set_cloudflare(&self, token: &str) -> Result<Cloudflare, Error> {
        self.put("/api/settings/cloudflare", json!({ "token": token }))
    }

    pub fn github(&self) -> Result<GitHub, Error> {
        self.get("/api/settings/github")
    }

    pub fn set_github(&self, token: &str) -> Result<GitHub, Error> {
        self.put("/api/settings/github", json!({ "token": token }))
    }

    pub fn remove_github(&self) -> Result<(), Error> {
        self.delete("/api/settings/github")
    }

    pub fn ssh_keys(&self) -> Result<Vec<SshKey>, Error> {
        self.get("/api/ssh-keys")
    }

    pub fn add_ssh_key(&self, name: &str, public_key: &str) -> Result<(), Error> {
        let body = json!({ "name": name, "public_key": public_key });
        self.post::<IgnoredAny>("/api/ssh-keys", body).map(|_| ())
    }

    pub fn remove_ssh_key(&self, id: &str) -> Result<(), Error> {
        self.delete(&format!("/api/ssh-keys/{id}"))
    }
}
