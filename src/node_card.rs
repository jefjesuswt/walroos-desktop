//! One node of the Nodes page: status, roles, what it runs, its meters and its problems.

use gpui_kit::component::alert::Alert;
use gpui_kit::component::label::Label;
use gpui_kit::component::progress::Progress;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Sizable, StyledExt, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::api::Node;
use crate::page::card;
use crate::status::status_tag;
use crate::when;

/// runs: the names of the apps and databases on it.
pub fn node_card(n: &Node, runs: &[String], cx: &App) -> impl IntoElement + use<> {
    let t = &n.telemetry;
    let status = status_tag(&n.status, n.status.clone());
    let mut facts = vec![format!("{}/{}", t.os, t.arch)];
    if !t.walroos.is_empty() {
        facts.push(format!("walroos {}", t.walroos));
    }
    facts.push(if t.podman.is_empty() {
        "no podman".into()
    } else {
        format!("podman {}", t.podman)
    });
    if t.containers >= 0 {
        facts.push(format!("{} containers", t.containers));
    }
    if let (Some(seen), "offline") = (&n.last_seen, n.status.as_str()) {
        facts.push(format!("last seen {}", when::short(seen)));
    }
    card(cx)
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(Label::new(n.name.clone()).font_semibold())
                .child(status)
                .children(
                    n.roles
                        .iter()
                        .map(|r| Tag::secondary().small().outline().child(r.clone())),
                )
                .child(div().flex_1())
                .child(
                    Label::new(facts.join(" · "))
                        .text_sm()
                        .text_color(cx.theme().muted_foreground),
                ),
        )
        .when(n.status != "pending", |el| {
            el.child(
                h_flex()
                    .gap_6()
                    .child(meter(
                        &n.id,
                        "cpu",
                        "CPU",
                        format!("{:.0}%", t.cpu_percent),
                        t.cpu_percent as f32,
                        cx,
                    ))
                    .child(meter(
                        &n.id,
                        "mem",
                        "Memory",
                        gb_of(t.mem_used_mb as f64 / 1024., t.mem_total_mb as f64 / 1024.),
                        pct(t.mem_used_mb, t.mem_total_mb),
                        cx,
                    ))
                    .child(meter(
                        &n.id,
                        "disk",
                        "Disk",
                        gb_of(t.disk_used_gb as f64, t.disk_total_gb as f64),
                        pct(t.disk_used_gb, t.disk_total_gb),
                        cx,
                    )),
            )
        })
        .child(
            Label::new(match runs.len() {
                0 => "Runs nothing yet.".to_string(),
                k => format!("Runs {k}: {}", runs.join(", ")),
            })
            .text_sm()
            .text_color(cx.theme().muted_foreground),
        )
        .children(t.problems.iter().map(|p| {
            Alert::warning(SharedString::from(format!("{}-{}", n.id, p.code)), p.fix.clone())
                .title(p.title.clone())
        }))
}

fn meter(node: &str, key: &str, label: &str, value: String, pct: f32, cx: &App) -> impl IntoElement {
    v_flex()
        .flex_1()
        .gap_1()
        .child(
            h_flex()
                .justify_between()
                .child(Label::new(label.to_string()).text_sm())
                .child(
                    Label::new(value)
                        .text_sm()
                        .text_color(cx.theme().muted_foreground),
                ),
        )
        .child(
            Progress::new(SharedString::from(format!("{node}-{key}")))
                .value(pct)
                .accessibility_label(label.to_string()),
        )
}

fn pct(used: u64, total: u64) -> f32 {
    if total == 0 {
        0.
    } else {
        (used as f32 / total as f32 * 100.).min(100.)
    }
}

fn gb_of(used: f64, total: f64) -> String {
    format!("{used:.1} / {total:.0} GB")
}
