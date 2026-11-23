//! An app and what hangs off it: its deployments, the release its node runs, its variables and
//! volumes. They mirror `web/src/types-apps.ts`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct App {
    pub name: String,
    pub node_name: String,
    pub project: String,
    pub description: String,
    pub repo_url: String, // '' when image is set; both '' = a draft with no source yet
    pub image: String,
    pub branch: String,
    pub dockerfile: String,
    #[serde(rename = "static")]
    pub static_build: Option<StaticBuild>, // a repo with no Dockerfile, built and served by walroos
    pub health: String,       // http (or ''), tcp, running
    pub command: Vec<String>, // replaces the image's
    pub context: String,      // the repo folder it builds from
    pub port: u16,
    pub health_path: String,
    pub domains: Vec<String>,
    pub volumes: Vec<Volume>,
    pub trigger: String, // manual, on_push, on_ci
    pub deploy_token: bool,
    pub last_deployment: Option<Deployment>,
    pub running: Option<Release>,
    pub node_down: bool,
}

impl App {
    /// No source yet: nothing to deploy.
    pub fn is_draft(&self) -> bool {
        self.repo_url.is_empty() && self.image.is_empty()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct StaticBuild {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub runtime: String, // '' picks bun with a bun lockfile, else node
    #[serde(skip_serializing_if = "String::is_empty")]
    pub install: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub build: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub publish: String,
    pub spa: bool,
}

/// A named podman volume (name) or a directory under the node's ~/walroos-data (host_path).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Volume {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub host_path: String,
    pub target: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub ro: bool,
}

impl Volume {
    /// Compose-like: name:/target for a podman volume, ./dir:/target for a directory under the
    /// node's ~/walroos-data; :ro at the end mounts it read-only.
    pub fn line(&self) -> String {
        let src = if self.name.is_empty() {
            format!("./{}", self.host_path)
        } else {
            self.name.clone()
        };
        let ro = if self.ro { ":ro" } else { "" };
        format!("{src}:{}{ro}", self.target)
    }

    pub fn parse(line: &str) -> Volume {
        let mut parts = line.splitn(3, ':');
        let (src, target, opt) = (
            parts.next().unwrap_or(""),
            parts.next().unwrap_or(""),
            parts.next(),
        );
        let mut v = Volume {
            target: target.into(),
            ro: opt == Some("ro"),
            ..Default::default()
        };
        match src.strip_prefix("./") {
            Some(dir) => v.host_path = dir.into(),
            None => v.name = src.into(),
        }
        v
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Deployment {
    pub id: String,
    pub app_name: Option<String>, // only in the all-apps list
    #[serde(default)]
    pub sha: String,
    #[serde(default)]
    pub image: String, // what it ran; a rollback needs it
    pub state: String, // queued, running, succeeded, failed
    pub error: Option<String>,
    #[serde(default)]
    pub created_at: String,
    pub finished_at: Option<String>,
    #[serde(default)]
    pub log: String, // only when getting one deployment
}

/// The release the node runs, as its agent reports it.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Release {
    pub container: String, // <app>-<deployment id>
    pub sha: String,
    pub image: String,
    pub state: String, // systemd ActiveState: active, activating, failed...
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
    pub secret: bool,
}

/// Where the forge reaches the hub's hooks (Tailscale Funnel), or why it can't.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Hooks {
    pub url: String,
    pub error: String,
}

/// A service of the project (or "shared") and the variables `${{service.KEY}}` can name.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Ref {
    pub service: String,
    pub keys: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::Volume;

    #[test]
    fn volumes_read_and_write_like_compose() {
        for line in ["data:/var/lib/data", "./photos:/photos:ro"] {
            assert_eq!(Volume::parse(line).line(), line);
        }
        assert_eq!(Volume::parse("./photos:/photos:ro").host_path, "photos");
        assert!(Volume::parse("data:/d").name == "data" && !Volume::parse("data:/d").ro);
    }
}

/// Builders the tests share: an app named web on prod-1 with a repo and nothing deployed yet.
#[cfg(test)]
pub mod fixtures {
    use super::{App, Deployment, Release};

    pub fn app() -> App {
        App {
            name: "web".into(),
            node_name: "prod-1".into(),
            repo_url: "https://github.com/example/web".into(),
            branch: "main".into(),
            ..Default::default()
        }
    }

    pub fn deployment(id: &str, state: &str, image: &str) -> Deployment {
        Deployment {
            id: id.into(),
            sha: format!("{id}abcdef"),
            image: image.into(),
            state: state.into(),
            error: None,
            created_at: "2026-11-22T10:00:00Z".into(),
            ..Default::default()
        }
    }

    /// The app's release running on the node, from deployment id.
    pub fn release(id: &str, state: &str) -> Release {
        Release {
            container: format!("web-{id}"),
            state: state.into(),
            ..Default::default()
        }
    }
}
