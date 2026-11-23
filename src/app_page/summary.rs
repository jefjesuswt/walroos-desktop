//! The General tab's read-only side: the app's source and run settings as a list of facts.

use gpui_kit::component::button::Button;
use gpui_kit::component::v_flex;
use gpui_kit::*;

use super::source::RUNTIMES;
use crate::api::{App as AppService, StaticBuild};
use crate::command;
use crate::ui::{facts, muted, section};

/// The source, read-only, with Edit when your role may change it.
pub fn summary(
    a: &AppService,
    can_edit: bool,
    on_edit: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let (title, sub) = match (&a.static_build, a.image.is_empty()) {
        (_, false) => (
            "Published image",
            "Pulled again on each deploy; a rollback runs the exact image it got then.",
        ),
        (Some(_), true) => (
            "Static site",
            "Cloned, built and served by walroos on each deploy: no Dockerfile.",
        ),
        (None, true) => (
            "Repository",
            "Cloned and built from its Dockerfile on each deploy.",
        ),
    };
    let edit = can_edit.then(|| {
        Button::new("edit-source")
            .outline()
            .label("Edit")
            .on_click(move |_, w, cx| on_edit(w, cx))
            .into_any_element()
    });
    section(title, sub, edit, cx)
        .child(facts(summary_rows(a), cx))
        .into_any_element()
}

fn or(v: &str, fallback: &str) -> String {
    if v.is_empty() { fallback.into() } else { v.into() }
}

fn summary_rows(a: &AppService) -> Vec<(&'static str, String)> {
    let mut rows = Vec::new();
    if !a.image.is_empty() {
        rows.push(("Image", a.image.clone()));
    } else {
        rows.push(("Repository", a.repo_url.clone()));
        rows.push(("Branch", or(&a.branch, "default branch")));
        match &a.static_build {
            Some(s) => rows.extend(static_rows(s)),
            None => rows.push(("Dockerfile", or(&a.dockerfile, "Dockerfile"))),
        }
    }
    let check = match a.health.as_str() {
        "tcp" => format!("TCP on port {} within 90 s", a.port),
        "running" => "keeps running for 5 s".into(),
        _ => format!("GET {} → 2xx/3xx within 90 s", or(&a.health_path, "/")),
    };
    let port = if a.port == 0 {
        "none (a worker)".into()
    } else {
        a.port.to_string()
    };
    rows.push(("Port", port));
    rows.push(("Health check", check));
    if !a.command.is_empty() {
        rows.push(("Command", command::join(&a.command)));
    }
    if !a.context.is_empty() {
        rows.push(("Build folder", a.context.clone()));
    }
    rows
}

fn static_rows(s: &StaticBuild) -> Vec<(&'static str, String)> {
    let runtime = RUNTIMES
        .iter()
        .find(|(r, _)| *r == s.runtime)
        .map_or("Auto", |(_, l)| l);
    let routing = if s.spa {
        "Single-page app (unknown paths → index.html)"
    } else {
        "Files only (unknown paths → 404.html)"
    };
    vec![
        ("Runtime", runtime.into()),
        ("Install", or(&s.install, "auto")),
        ("Build command", or(&s.build, "the build script, if any")),
        ("Publish", or(&s.publish, "found after the build")),
        ("Routing", routing.into()),
    ]
}

/// For a role that can't set the source of a draft.
pub fn no_source(cx: &App) -> AnyElement {
    v_flex()
        .child(muted("No source yet. Someone who can deploy apps sets it.", cx))
        .into_any_element()
}
