//! The Projects page: each project with its apps and databases and what each one is doing.

use gpui_kit::component::label::Label;
use gpui_kit::component::{ActiveTheme, StyledExt, h_flex, v_flex};
use gpui_kit::*;

use crate::api::{App as AppService, Database, Project};
use crate::page::{Spec, card, empty};
use crate::status::{State, app_state, db_state, engine_label};

pub fn spec() -> Spec<Vec<Project>> {
    Spec {
        title: "Projects",
        fetch: |hub| hub.projects(),
        body: |projects, cx| body(projects, cx),
    }
}

fn body(projects: &[Project], cx: &App) -> AnyElement {
    if projects.is_empty() {
        return empty("No projects yet. Create one in the web panel.", cx);
    }
    v_flex()
        .gap_3()
        .children(projects.iter().map(|p| project_card(p, cx)))
        .into_any_element()
}

/// One row of a project: an app or a database, reduced to what the row shows.
struct Service {
    kind: &'static str,
    name: String,
    what: String,
    node: String,
    state: State,
    trouble: Option<String>, // the last deploy's or backup's error
}

fn app_service(a: &AppService) -> Service {
    let failed = a.last_deployment.as_ref().filter(|d| d.state == "failed");
    let mut what = source(a);
    if let Some(domain) = a.domains.first() {
        what = format!("{what} · {domain}");
    }
    Service {
        kind: "App",
        name: a.name.clone(),
        what,
        node: a.node_name.clone(),
        state: app_state(a),
        trouble: failed.map(|d| {
            format!(
                "Last deploy failed: {}",
                d.error.as_deref().unwrap_or("no error saved")
            )
        }),
    }
}

fn db_service(d: &Database) -> Service {
    Service {
        kind: "Database",
        name: d.name.clone(),
        what: format!("{} {}", engine_label(&d.engine), d.version),
        node: d.node_name.clone(),
        state: db_state(d),
        trouble: d
            .last_backup_error
            .as_ref()
            .map(|e| format!("Last backup failed: {e}")),
    }
}

/// Where an app comes from: its image, or its repo (owner/name) and branch.
fn source(a: &AppService) -> String {
    if !a.image.is_empty() {
        return a.image.clone();
    }
    if a.repo_url.is_empty() {
        return "no source yet".into();
    }
    let repo = a.repo_url.trim_end_matches('/').trim_end_matches(".git");
    let short: Vec<&str> = repo.rsplit('/').take(2).collect();
    let repo = if short.len() == 2 {
        format!("{}/{}", short[1], short[0])
    } else {
        repo.to_string()
    };
    if a.branch.is_empty() {
        repo
    } else {
        format!("{repo}@{}", a.branch)
    }
}

fn project_card(p: &Project, cx: &App) -> impl IntoElement {
    let services: Vec<Service> = p
        .apps
        .iter()
        .map(app_service)
        .chain(p.databases.iter().map(db_service))
        .collect();
    let running = services.iter().filter(|s| s.state.running()).count();
    let muted = cx.theme().muted_foreground;
    card(cx)
        .child(
            h_flex()
                .gap_3()
                .items_baseline()
                .child(Label::new(p.name.clone()).text_lg().font_semibold())
                .child(Label::new(p.description.clone()).text_sm().text_color(muted))
                .child(div().flex_1())
                .child(
                    Label::new(format!("{running} of {} running", services.len()))
                        .text_sm()
                        .text_color(muted),
                ),
        )
        .children(
            services
                .is_empty()
                .then(|| Label::new("No services yet.").text_sm().text_color(muted)),
        )
        .children(services.into_iter().map(|s| service_row(s, cx)))
}

fn service_row(s: Service, cx: &App) -> impl IntoElement {
    let muted = cx.theme().muted_foreground;
    v_flex()
        .gap_1()
        .pt_3()
        .border_t_1()
        .border_color(cx.theme().border)
        .child(
            h_flex()
                .gap_3()
                .items_center()
                .child(Label::new(s.kind).text_xs().text_color(muted).w(px(64.)))
                .child(Label::new(s.name).font_medium())
                .child(s.state.tag())
                .child(Label::new(s.what).text_sm().text_color(muted).truncate())
                .child(div().flex_1())
                .child(Label::new(format!("on {}", s.node)).text_sm().text_color(muted)),
        )
        .children(
            s.trouble
                .map(|t| Label::new(t).text_sm().text_color(cx.theme().danger).pl(px(76.))),
        )
}

#[cfg(test)]
mod tests {
    use super::{AppService, source};

    fn app(repo: &str, image: &str, branch: &str) -> AppService {
        AppService {
            name: "web".into(),
            node_name: "prod-1".into(),
            repo_url: repo.into(),
            image: image.into(),
            branch: branch.into(),
            domains: vec![],
            last_deployment: None,
            running: None,
            node_down: false,
        }
    }

    #[test]
    fn says_where_an_app_comes_from() {
        assert_eq!(
            source(&app("https://github.com/example/web.git", "", "main")),
            "example/web@main"
        );
        assert_eq!(
            source(&app("", "ghcr.io/example/api:1.2", "")),
            "ghcr.io/example/api:1.2"
        );
        assert_eq!(source(&app("", "", "")), "no source yet");
    }
}
