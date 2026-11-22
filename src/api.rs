//! The parts of the hub's API this app reads. They mirror `web/src/types.ts` in the walroos
//! repo (which mirrors the Go models); fields the app doesn't show are left out.

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct User {
    pub login: String,
    pub role: String, // owner, deployer, developer
}

impl User {
    /// Owners and deployers deploy, stop and roll back apps; developers only look at them.
    pub fn manages_apps(&self) -> bool {
        self.role == "owner" || self.role == "deployer"
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub roles: Vec<String>,
    pub status: String, // pending (never reported), online, offline
    #[serde(default)]
    pub telemetry: Telemetry,
    pub last_seen: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Telemetry {
    pub os: String,
    pub arch: String,
    pub cpu_percent: f64,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    pub disk_used_gb: u64,
    pub disk_total_gb: u64,
    pub podman: String, // empty when podman isn't installed
    pub walroos: String,
    pub containers: i64, // -1 when unknown
    pub problems: Vec<NodeProblem>,
}

/// Something wrong on a node that walroos can't fix by itself, with the fix in words.
#[derive(Clone, Debug, Deserialize)]
pub struct NodeProblem {
    pub code: String,
    pub title: String,
    pub fix: String,
}

/// A group of apps and databases sharing a podman network on their node.
#[derive(Clone, Debug, Deserialize)]
pub struct Project {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub apps: Vec<App>,
    #[serde(default)]
    pub databases: Vec<Database>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct App {
    pub name: String,
    pub node_name: String,
    #[serde(default)]
    pub repo_url: String, // '' when image is set; both '' = a draft with no source yet
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub branch: String,
    #[serde(default)]
    pub domains: Vec<String>,
    pub last_deployment: Option<Deployment>,
    pub running: Option<Release>,
    #[serde(default)]
    pub node_down: bool,
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
#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    pub container: String, // <app>-<deployment id>
    pub state: String,     // systemd ActiveState: active, activating, failed...
}

#[derive(Clone, Debug, Deserialize)]
pub struct Database {
    pub name: String,
    pub node_name: String,
    pub engine: String,
    pub version: String,
    pub state: Option<String>, // systemd ActiveState on the node; 'missing' if the node doesn't have it
    #[serde(default)]
    pub node_down: bool,
    pub last_backup_error: Option<String>,
}

impl Node {
    /// How urgent the node is to look at: problems first, then offline, pending, online.
    fn urgency(&self) -> u8 {
        match (self.telemetry.problems.is_empty(), self.status.as_str()) {
            (false, _) => 0,
            (true, "offline") => 1,
            (true, "pending") => 2,
            _ => 3,
        }
    }
}

/// Nodes in the order the list shows them: the ones that need you first, then by name.
pub fn sort_nodes(nodes: &mut [Node]) {
    nodes.sort_by(|a, b| a.urgency().cmp(&b.urgency()).then_with(|| a.name.cmp(&b.name)));
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
            image: String::new(),
            branch: "main".into(),
            domains: vec![],
            last_deployment: None,
            running: None,
            node_down: false,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The shape GET /api/nodes answers with (internal/fleet/models.go).
    const NODES: &str = r#"[
      {"id":"n1","name":"prod-1","roles":["prod"],"address":"100.64.0.2","status":"online",
       "telemetry":{"hostname":"prod-1","os":"linux","arch":"amd64","cpu_percent":12.5,
         "mem_used_mb":2048,"mem_total_mb":8192,"disk_used_gb":40,"disk_total_gb":100,
         "podman":"5.4.0","walroos":"v0.2.0","containers":7},
       "created_at":"2026-11-01T10:00:00Z","last_seen":"2026-11-22T10:00:00Z"},
      {"id":"n2","name":"build-1","roles":["build","dev"],"address":"100.64.0.3","status":"online",
       "telemetry":{"os":"linux","arch":"arm64","containers":-1,
         "problems":[{"code":"quadlet","title":"Quadlet is missing","fix":"Install podman 4.4 or newer."}]},
       "created_at":"2026-11-01T10:00:00Z","last_seen":"2026-11-22T10:00:00Z"},
      {"id":"n3","name":"dev-1","roles":[],"address":"","status":"pending",
       "telemetry":{},"created_at":"2026-11-01T10:00:00Z","last_seen":null},
      {"id":"n4","name":"old-1","roles":["dev"],"address":"100.64.0.4","status":"offline",
       "telemetry":{},"created_at":"2026-11-01T10:00:00Z","last_seen":"2026-11-20T08:30:00Z"}
    ]"#;

    #[test]
    fn reads_nodes() {
        let nodes: Vec<Node> = serde_json::from_str(NODES).unwrap();
        assert_eq!(nodes.len(), 4);
        assert_eq!(nodes[0].telemetry.mem_total_mb, 8192);
        assert_eq!(nodes[1].telemetry.containers, -1);
        assert_eq!(nodes[1].telemetry.problems[0].title, "Quadlet is missing");
        assert!(nodes[2].last_seen.is_none());
    }

    #[test]
    fn sorts_nodes_that_need_you_first() {
        let mut nodes: Vec<Node> = serde_json::from_str(NODES).unwrap();
        sort_nodes(&mut nodes);
        let names: Vec<_> = nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, ["build-1", "old-1", "dev-1", "prod-1"]);
    }
}
