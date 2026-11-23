//! What an app's page asks the hub, past the list: its setup, variables, domains, volumes and
//! how it deploys by itself. Paths as in `web/src/api.ts`.

use serde::Deserialize;
use serde::de::IgnoredAny;
use serde_json::{Value, json};

use super::{Error, Hub};
use crate::api::{App, EnvVar, Hooks, Ref, Volume};

#[derive(Deserialize)]
struct Secret {
    #[serde(alias = "token")]
    secret: String,
}

impl Hub {
    pub fn app(&self, name: &str) -> Result<App, Error> {
        self.get(&format!("/api/apps/{name}"))
    }

    /// src is the source and run settings, as the web panel's SourceCard sends them.
    pub fn set_source(&self, name: &str, src: Value) -> Result<App, Error> {
        self.put(&format!("/api/apps/{name}/source"), src)
    }

    pub fn app_env(&self, name: &str) -> Result<Vec<EnvVar>, Error> {
        self.get(&format!("/api/apps/{name}/env"))
    }

    /// Replaces the whole set.
    pub fn set_app_env(&self, name: &str, vars: &[EnvVar]) -> Result<(), Error> {
        self.put::<IgnoredAny>(&format!("/api/apps/{name}/env"), json!(vars))
            .map(|_| ())
    }

    pub fn set_domains(&self, name: &str, domains: &[String]) -> Result<(), Error> {
        self.put::<IgnoredAny>(&format!("/api/apps/{name}/domains"), json!(domains))
            .map(|_| ())
    }

    pub fn set_volumes(&self, name: &str, volumes: &[Volume]) -> Result<(), Error> {
        self.put::<IgnoredAny>(&format!("/api/apps/{name}/volumes"), json!(volumes))
            .map(|_| ())
    }

    /// trigger is manual, on_push or on_ci.
    pub fn set_trigger(&self, name: &str, trigger: &str) -> Result<(), Error> {
        let body = json!({ "trigger": trigger });
        self.put::<IgnoredAny>(&format!("/api/apps/{name}/trigger"), body)
            .map(|_| ())
    }

    pub fn webhook_secret(&self, name: &str) -> Result<String, Error> {
        self.get::<Secret>(&format!("/api/apps/{name}/webhook-secret"))
            .map(|s| s.secret)
    }

    pub fn rotate_webhook_secret(&self, name: &str) -> Result<String, Error> {
        self.post::<Secret>(&format!("/api/apps/{name}/webhook-secret"), json!({}))
            .map(|s| s.secret)
    }

    /// A new deploy token for CI, shown this once. It replaces the old one.
    pub fn new_deploy_token(&self, name: &str) -> Result<String, Error> {
        self.post::<Secret>(&format!("/api/apps/{name}/deploy-token"), json!({}))
            .map(|s| s.secret)
    }

    pub fn revoke_deploy_token(&self, name: &str) -> Result<(), Error> {
        self.delete(&format!("/api/apps/{name}/deploy-token"))
    }

    pub fn hooks(&self) -> Result<Hooks, Error> {
        self.get("/api/hooks")
    }

    pub fn project_refs(&self, project: &str) -> Result<Vec<Ref>, Error> {
        self.get(&format!("/api/projects/{project}/refs"))
    }

    /// purge also deletes its named volumes.
    pub fn delete_app(&self, name: &str, purge: bool) -> Result<(), Error> {
        let purge = if purge { "?purge=1" } else { "" };
        self.delete(&format!("/api/apps/{name}{purge}"))
    }
}
