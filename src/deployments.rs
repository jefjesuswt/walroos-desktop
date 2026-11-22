//! The Deployments page: every deploy of every app, newest first. A row opens its build log.

use gpui_kit::component::label::Label;
use gpui_kit::component::{ActiveTheme, StyledExt, h_flex};
use gpui_kit::*;

use crate::api::Deployment;
use crate::log_view::{self, Source};
use crate::page::{Ctx, Spec, card, empty};
use crate::status::status_tag;
use crate::when;

/// How many the page shows: the hub keeps more, but the recent ones are what you look for.
const SHOWN: usize = 100;

pub fn spec() -> Spec<Vec<Deployment>> {
    Spec {
        title: "Deployments",
        fetch: |hub| Ok(hub.all_deployments()?.into_iter().take(SHOWN).collect()),
        body: |list, ctx, cx| body(list, ctx, cx),
        settling: |list| list.iter().any(|d| d.state == "queued" || d.state == "running"),
    }
}

fn body(list: &[Deployment], ctx: &Ctx, cx: &App) -> AnyElement {
    if list.is_empty() {
        return empty("No deployments yet. Deploy an app from the Projects tab.", cx);
    }
    card(cx)
        .gap_0()
        .p_0()
        .children(list.iter().map(|d| row(d, ctx, cx)))
        .into_any_element()
}

fn row(d: &Deployment, ctx: &Ctx, cx: &App) -> impl IntoElement {
    let app = d.app_name.clone().unwrap_or_default();
    let muted = cx.theme().muted_foreground;
    let took = when::took(&d.created_at, d.finished_at.as_deref()).unwrap_or_default();
    let source = Source::Deploy {
        app: app.clone(),
        id: d.id.clone(),
    };
    let link = ctx.link.clone();
    h_flex()
        .id(SharedString::from(format!("deploy-{}", d.id)))
        .gap_3()
        .items_center()
        .px_4()
        .py_2()
        .border_b_1()
        .border_color(cx.theme().border)
        .cursor_pointer()
        .hover(|s| s.bg(cx.theme().list_hover))
        .on_click(move |_, window, cx| log_view::open(source.clone(), link.clone(), window, cx))
        .child(Label::new(app).font_medium().w(px(160.)).truncate())
        .child(h_flex().w(px(96.)).child(status_tag(&d.state, d.state.clone())))
        .child(
            Label::new(d.sha.chars().take(7).collect::<String>())
                .font_family(cx.theme().mono_font_family.clone())
                .text_sm()
                .w(px(72.)),
        )
        .child(
            Label::new(d.error.clone().unwrap_or_default())
                .text_sm()
                .text_color(cx.theme().danger)
                .truncate()
                .flex_1(),
        )
        .child(
            Label::new(when::short(&d.created_at))
                .text_sm()
                .text_color(muted)
                .w(px(124.)),
        )
        .child(Label::new(took).text_sm().text_color(muted).w(px(64.)))
}
