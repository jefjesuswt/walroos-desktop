//! Rolling an app back: pick one of its earlier releases and run that image again.

use gpui_kit::component::button::Button;
use gpui_kit::component::label::Label;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme, Sizable, WindowExt, h_flex, v_flex};
use gpui_kit::*;

use crate::actions::{follow, run};
use crate::api::{App as AppService, Deployment};
use crate::page::Ctx;
use crate::when;

/// The releases an app can go back to: ones that deployed fine and kept their image, other
/// than the one running. The newest first, as the hub lists them. Same rule as the web panel's
/// DeploymentModal.tsx.
pub fn targets(a: &AppService, deployments: Vec<Deployment>) -> Vec<Deployment> {
    let current = a.running.as_ref().map(|r| r.container.as_str());
    deployments
        .into_iter()
        .filter(|d| d.state == "succeeded" && !d.image.is_empty())
        .filter(|d| current != Some(format!("{}-{}", a.name, d.id).as_str()))
        .take(8)
        .collect()
}

/// Asks the hub for the app's deployments, then lets you pick one to go back to.
pub fn open(a: &AppService, ctx: &Ctx, window: &mut Window, cx: &mut App) {
    let (a, ctx, link, handle) = (a.clone(), ctx.clone(), ctx.link.clone(), window.window_handle());
    let name = a.name.clone();
    let task = cx.background_spawn(async move { link.call(|hub| hub.deployments(&name)) });
    cx.spawn(async move |cx| {
        let result = task.await;
        cx.update_window(handle, |_, window, cx| match result {
            Ok(list) => dialog(a.clone(), targets(&a, list), ctx, window, cx),
            Err(e) => window.push_notification(Notification::error(e.to_string()), cx),
        })
        .ok();
    })
    .detach();
}

fn dialog(a: AppService, targets: Vec<Deployment>, ctx: Ctx, window: &mut Window, cx: &mut App) {
    if targets.is_empty() {
        let note = format!("{} has no earlier release to roll back to.", a.name);
        return window.push_notification(Notification::info(note), cx);
    }
    window.open_alert_dialog(cx, move |dialog, _, cx| {
        let rows = targets.iter().map(|d| row(&a, d, &ctx, cx));
        dialog
            .title(format!("Roll back {}", a.name))
            .description("It runs the image that release built, with today's environment.")
            .child(v_flex().gap_2().children(rows))
            .ok_text("Close")
    });
}

fn row(a: &AppService, d: &Deployment, ctx: &Ctx, cx: &App) -> impl IntoElement {
    let (name, id, ctx) = (a.name.clone(), d.id.clone(), ctx.clone());
    let what = if d.sha.is_empty() {
        d.image.clone()
    } else {
        d.sha.chars().take(7).collect()
    };
    let done = format!("Rolling back {name} to {what}");
    h_flex()
        .gap_3()
        .items_center()
        .child(Label::new(what).font_family(cx.theme().mono_font_family.clone()))
        .child(
            Label::new(when::short(&d.created_at))
                .text_sm()
                .text_color(cx.theme().muted_foreground),
        )
        .child(div().flex_1())
        .child(
            Button::new(SharedString::from(format!("rollback-{id}")))
                .small()
                .outline()
                .label("Roll back")
                .on_click(move |_, window, cx| {
                    window.close_dialog(cx);
                    let (n, id) = (name.clone(), id.clone());
                    run(
                        &ctx,
                        done.clone(),
                        move |hub| hub.rollback(&n, &id),
                        follow(&ctx, &name),
                        window,
                        cx,
                    );
                }),
        )
}

#[cfg(test)]
mod tests {
    use super::targets;
    use crate::api::App;
    use crate::api::fixtures::{app, deployment, release};

    #[test]
    fn rolls_back_only_to_good_releases_other_than_the_running_one() {
        let web = App {
            running: Some(release("d4", "active")),
            ..app()
        };
        let list = vec![
            deployment("d4", "succeeded", "registry/web:d4"), // running now
            deployment("d3", "failed", ""),
            deployment("d2", "succeeded", "registry/web:d2"),
            deployment("d1", "succeeded", ""), // its image is gone
        ];
        let ids: Vec<_> = targets(&web, list).into_iter().map(|d| d.id).collect();
        assert_eq!(ids, ["d2"]);
    }
}
