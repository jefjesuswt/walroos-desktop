//! What each resource is doing, in one place, like `web/src/status.ts` in the walroos repo: a
//! service shows the same state here as in the web panel.

use gpui_kit::component::Sizable;
use gpui_kit::component::tag::Tag;
use gpui_kit::*;

use crate::api::{App, Database};

/// A raw state and the words for it.
#[derive(Debug, PartialEq)]
pub struct State {
    pub raw: &'static str,
    pub label: String,
}

fn state(raw: &'static str, label: impl Into<String>) -> State {
    State {
        raw,
        label: label.into(),
    }
}

impl State {
    pub fn running(&self) -> bool {
        self.raw == "running"
    }

    pub fn tag(&self) -> Tag {
        status_tag(self.raw, self.label.clone())
    }
}

/// A small badge for a raw state, colored by what it means.
pub fn status_tag(raw: &str, label: impl Into<SharedString>) -> Tag {
    match raw {
        "online" | "running" | "succeeded" | "active" => Tag::success(),
        "pending" | "creating" | "deleting" | "deploying" | "activating" => Tag::warning(),
        "offline" | "failed" | "missing" => Tag::danger(),
        _ => Tag::secondary(),
    }
    .small()
    .outline()
    .child(label.into())
}

/// What the app is doing, from the node's live release first: a deployment's state is only what
/// the hub saw (it can say failed for a deploy the node finished after a hub restart).
pub fn app_state(a: &App) -> State {
    if a.node_down {
        return state("offline", "node offline");
    }
    if a.is_draft() {
        return state("stopped", "no source yet");
    }
    let d = a.last_deployment.as_ref();
    match (d.map(|d| d.state.as_str()), &a.running) {
        (Some("queued"), _) => state("deploying", "queued"),
        (Some("running"), _) => state("deploying", "deploying"),
        (_, Some(r)) if r.state == "active" => state("running", "running"),
        (_, Some(r)) if r.state == "inactive" => state("stopped", "stopped"),
        (_, Some(r)) if r.state == "failed" => state("failed", "failed"),
        (_, Some(r)) => state("activating", r.state.clone()),
        (Some("failed"), None) => state("failed", "deploy failed"),
        _ => state("stopped", "not deployed"),
    }
}

/// What the database is doing: the node's systemd state, or why we don't know it.
pub fn db_state(d: &Database) -> State {
    if d.node_down {
        return state("offline", "node offline");
    }
    match d.state.as_deref().unwrap_or("") {
        "active" => state("running", "running"),
        "activating" => state("activating", "starting"),
        "failed" => state("failed", "failed"),
        "missing" => state("missing", "missing"),
        "" => state("stopped", "unknown"),
        other => state("stopped", other.to_string()),
    }
}

pub fn engine_label(engine: &str) -> &str {
    match engine {
        "postgres" => "PostgreSQL",
        "mysql" => "MySQL",
        "mariadb" => "MariaDB",
        "redis" => "Redis",
        "valkey" => "Valkey",
        "mongo" => "MongoDB",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::{app_state, db_state, state};
    use crate::api::{App, Database, fixtures};

    fn app(source: bool, deploy: Option<&str>, running: Option<&str>) -> App {
        App {
            repo_url: if source {
                fixtures::app().repo_url
            } else {
                String::new()
            },
            last_deployment: deploy.map(|s| fixtures::deployment("d1", s, "")),
            running: running.map(|s| fixtures::release("d1", s)),
            ..fixtures::app()
        }
    }

    #[test]
    fn app_states() {
        let label = |a: App| app_state(&a).label;
        assert_eq!(label(app(false, None, None)), "no source yet");
        assert_eq!(label(app(true, None, None)), "not deployed");
        assert_eq!(label(app(true, Some("queued"), Some("active"))), "queued");
        assert_eq!(label(app(true, Some("running"), Some("active"))), "deploying");
        // The node's release wins over what the hub saw of the deployment.
        assert_eq!(label(app(true, Some("failed"), Some("active"))), "running");
        assert_eq!(label(app(true, Some("failed"), None)), "deploy failed");
        assert_eq!(label(app(true, Some("succeeded"), Some("inactive"))), "stopped");
        let mut down = app(true, Some("succeeded"), Some("active"));
        down.node_down = true;
        assert_eq!(app_state(&down), state("offline", "node offline"));
    }

    #[test]
    fn db_states() {
        let db = |s: Option<&str>| Database {
            name: "db".into(),
            node_name: "prod-1".into(),
            engine: "postgres".into(),
            version: "17".into(),
            state: s.map(Into::into),
            node_down: false,
            last_backup_error: None,
        };
        assert!(db_state(&db(Some("active"))).running());
        assert_eq!(db_state(&db(Some("activating"))).label, "starting");
        assert_eq!(db_state(&db(None)).label, "unknown");
    }
}
