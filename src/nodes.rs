//! The Nodes page: the fleet at a glance (totals), every node with its health and what it runs,
//! the ones that need you first, and the latest deployments.

use gpui_kit::component::label::Label;
use gpui_kit::component::{StyledExt, h_flex, v_flex};
use gpui_kit::*;

use crate::api::{Deployment, Node, Project, sort_nodes};
use crate::deployments;
use crate::node_card::node_card;
use crate::page::{Ctx, Spec, card, empty};
use crate::status::{app_state, db_state};
use crate::ui::{muted, section};
use crate::when;

/// How many of the latest deployments the page lists.
const LATEST: usize = 5;

pub struct Overview {
    nodes: Vec<Node>,
    projects: Vec<Project>,
    deploys: Vec<Deployment>, // newest first, every app's
}

pub fn spec() -> Spec<Overview> {
    Spec {
        title: "Nodes",
        fetch: |hub| {
            let mut nodes = hub.nodes()?;
            sort_nodes(&mut nodes);
            Ok(Overview {
                nodes,
                projects: hub.projects()?,
                deploys: hub.all_deployments()?,
            })
        },
        body: |o, ctx, cx| body(o, ctx, cx),
        settling: |o| {
            o.deploys
                .iter()
                .any(|d| d.state == "queued" || d.state == "running")
        },
    }
}

fn body(o: &Overview, ctx: &Ctx, cx: &App) -> AnyElement {
    if o.nodes.is_empty() {
        return empty("No nodes yet. Join one with `walroos node join`.", cx);
    }
    v_flex()
        .gap_4()
        .child(totals(o, cx))
        .children(o.nodes.iter().map(|n| node_card(n, &runs_on(o, &n.name), cx)))
        .child(latest(o, ctx, cx))
        .into_any_element()
}

/// The names of the apps and databases on a node.
fn runs_on(o: &Overview, node: &str) -> Vec<String> {
    let apps = o
        .projects
        .iter()
        .flat_map(|p| &p.apps)
        .filter(|a| a.node_name == node);
    let dbs = o
        .projects
        .iter()
        .flat_map(|p| &p.databases)
        .filter(|d| d.node_name == node);
    apps.map(|a| a.name.clone())
        .chain(dbs.map(|d| d.name.clone()))
        .collect()
}

fn totals(o: &Overview, cx: &App) -> impl IntoElement + use<> {
    let online = o.nodes.iter().filter(|n| n.status == "online").count();
    let apps: Vec<_> = o.projects.iter().flat_map(|p| &p.apps).collect();
    let dbs: Vec<_> = o.projects.iter().flat_map(|p| &p.databases).collect();
    let today = when::now() - 24 * 3600;
    let recent: Vec<_> = o
        .deploys
        .iter()
        .filter(|d| when::parse(&d.created_at).is_some_and(|t| t >= today))
        .collect();
    let failed = recent.iter().filter(|d| d.state == "failed").count();
    h_flex()
        .gap_3()
        .child(tile("Nodes", online, format!("online of {}", o.nodes.len()), cx))
        .child(tile(
            "Apps",
            apps.iter().filter(|a| app_state(a).running()).count(),
            format!("running of {}", apps.len()),
            cx,
        ))
        .child(tile(
            "Databases",
            dbs.iter().filter(|d| db_state(d).running()).count(),
            format!("running of {}", dbs.len()),
            cx,
        ))
        .child(tile(
            "Deploys",
            recent.len(),
            format!("in the last 24 h · {failed} failed"),
            cx,
        ))
}

fn tile(label: &'static str, n: usize, sub: String, cx: &App) -> impl IntoElement + use<> {
    card(cx)
        .flex_1()
        .gap_1()
        .child(muted(label, cx))
        .child(Label::new(n.to_string()).text_2xl().font_semibold())
        .child(muted(sub, cx))
}

fn latest(o: &Overview, ctx: &Ctx, cx: &App) -> impl IntoElement + use<> {
    let list = if o.deploys.is_empty() {
        muted("Nothing deployed yet.", cx).into_any_element()
    } else {
        let rows = o
            .deploys
            .iter()
            .take(LATEST)
            .map(|d| deployments::row(d, d.app_name.as_deref().unwrap_or_default(), true, ctx, cx));
        v_flex().children(rows).into_any_element()
    };
    section(
        "Latest deployments",
        "Every app's, newest first. Click one to read its log; the Deployments tab has them all.",
        None,
        cx,
    )
    .child(list)
}
