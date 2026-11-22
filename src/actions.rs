//! What you can do to an app from its row: deploy, start, stop, restart and roll back. Each
//! runs on the hub in the background, says how it went in a notification and refreshes the page.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_kit::component::label::Label;
use gpui_kit::component::menu::{DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, WindowExt, h_flex, v_flex};
use gpui_kit::*;

use crate::api::{App as AppService, Deployment};
use crate::hub::{Error, Hub};
use crate::page::Ctx;

/// Runs change on the hub off the UI thread; then tells you `done` or what went wrong, and
/// refreshes the page so it shows the new state.
fn run(
    ctx: &Ctx,
    done: String,
    change: impl FnOnce(&Hub) -> Result<(), Error> + Send + 'static,
    window: &mut Window,
    cx: &mut App,
) {
    let (link, refresh, handle) = (ctx.link.clone(), ctx.refresh.clone(), window.window_handle());
    let task = cx.background_spawn(async move { link.call(change) });
    cx.spawn(async move |cx| {
        let result = task.await;
        cx.update_window(handle, |_, window, cx| {
            let note = match result {
                Ok(()) => Notification::success(done),
                Err(e) => Notification::error(e.to_string()),
            };
            window.push_notification(note, cx);
            refresh(cx);
        })
        .ok();
    })
    .detach();
}

/// The ⋮ button of an app's row, with what its state allows.
pub fn app_menu(a: &AppService, ctx: &Ctx) -> AnyElement {
    let (a, ctx) = (a.clone(), ctx.clone());
    Button::new(SharedString::from(format!("menu-{}", a.name)))
        .ghost()
        .small()
        .icon(Icon::new(IconName::EllipsisVertical))
        .dropdown_menu(move |menu, _, _| app_items(menu, &a, &ctx))
        .into_any_element()
}

fn app_items(mut menu: PopupMenu, a: &AppService, ctx: &Ctx) -> PopupMenu {
    let has_source = !a.repo_url.is_empty() || !a.image.is_empty();
    let deploy = if a.running.is_some() { "Redeploy" } else { "Deploy" };
    menu = menu.item(
        item(deploy, a, ctx, |a, ctx, window, cx| {
            let name = a.name.clone();
            run(
                ctx,
                format!("Deploying {}", a.name),
                move |hub| hub.deploy(&name),
                window,
                cx,
            )
        })
        .disabled(!has_source),
    );
    match &a.running {
        Some(r) if !a.node_down && r.state == "inactive" => {
            menu = menu.item(item("Start", a, ctx, |a, ctx, w, cx| {
                power(a, ctx, "start", w, cx)
            }));
        }
        Some(_) if !a.node_down => {
            menu = menu
                .item(item("Restart", a, ctx, |a, ctx, w, cx| {
                    power(a, ctx, "restart", w, cx)
                }))
                .item(item("Stop", a, ctx, confirm_stop));
        }
        _ => {}
    }
    if has_source && !a.node_down {
        menu = menu.separator().item(item("Roll back…", a, ctx, open_rollback));
    }
    menu
}

/// A menu item that does f to the app.
fn item(
    label: &'static str,
    a: &AppService,
    ctx: &Ctx,
    f: fn(&AppService, &Ctx, &mut Window, &mut App),
) -> PopupMenuItem {
    let (a, ctx) = (a.clone(), ctx.clone());
    PopupMenuItem::new(label).on_click(move |_, window, cx| f(&a, &ctx, window, cx))
}

fn power(a: &AppService, ctx: &Ctx, action: &'static str, window: &mut Window, cx: &mut App) {
    let done = match action {
        "stop" => "stopped",
        "start" => "started",
        _ => "restarted",
    };
    let name = a.name.clone();
    run(
        ctx,
        format!("{} {done}", a.name),
        move |hub| hub.power(&name, action),
        window,
        cx,
    );
}

/// Stopping takes the app offline until someone starts it, so it asks first.
fn confirm_stop(a: &AppService, ctx: &Ctx, window: &mut Window, cx: &mut App) {
    let (a, ctx) = (a.clone(), ctx.clone());
    let domains = if a.domains.is_empty() {
        String::new()
    } else {
        format!(" on {}", a.domains.join(", "))
    };
    let text = format!(
        "It stops answering{domains}. It stays stopped, even when {} restarts, until you start it or deploy it again.",
        a.node_name
    );
    window.open_alert_dialog(cx, move |dialog, _, _| {
        let (a, ctx) = (a.clone(), ctx.clone());
        dialog
            .title(format!("Stop {}?", a.name))
            .description(text.clone())
            .ok_text("Stop")
            .ok_variant(ButtonVariant::Danger)
            .show_cancel(true)
            .on_ok(move |_, window, cx| {
                power(&a, &ctx, "stop", window, cx);
                true
            })
    });
}

/// The releases an app can go back to: ones that deployed fine and kept their image, other
/// than the one running. The newest first, as the hub lists them.
pub fn rollback_targets(a: &AppService, deployments: Vec<Deployment>) -> Vec<Deployment> {
    let current = a.running.as_ref().map(|r| r.container.as_str());
    deployments
        .into_iter()
        .filter(|d| d.state == "succeeded" && !d.image.is_empty())
        .filter(|d| current != Some(format!("{}-{}", a.name, d.id).as_str()))
        .take(8)
        .collect()
}

/// Asks the hub for the app's deployments, then lets you pick one to go back to.
fn open_rollback(a: &AppService, ctx: &Ctx, window: &mut Window, cx: &mut App) {
    let (a, ctx, link, handle) = (a.clone(), ctx.clone(), ctx.link.clone(), window.window_handle());
    let name = a.name.clone();
    let task = cx.background_spawn(async move { link.call(|hub| hub.deployments(&name)) });
    cx.spawn(async move |cx| {
        let result = task.await;
        cx.update_window(handle, |_, window, cx| match result {
            Ok(list) => rollback_dialog(a.clone(), rollback_targets(&a, list), ctx, window, cx),
            Err(e) => window.push_notification(Notification::error(e.to_string()), cx),
        })
        .ok();
    })
    .detach();
}

fn rollback_dialog(a: AppService, targets: Vec<Deployment>, ctx: Ctx, window: &mut Window, cx: &mut App) {
    if targets.is_empty() {
        let note = format!("{} has no earlier release to roll back to.", a.name);
        return window.push_notification(Notification::info(note), cx);
    }
    window.open_alert_dialog(cx, move |dialog, _, cx| {
        let rows = targets.iter().map(|d| rollback_row(&a, d, &ctx, cx));
        dialog
            .title(format!("Roll back {}", a.name))
            .description("It runs the image that release built, with today's environment.")
            .child(v_flex().gap_2().children(rows))
            .ok_text("Close")
    });
}

fn rollback_row(a: &AppService, d: &Deployment, ctx: &Ctx, cx: &App) -> impl IntoElement {
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
            Label::new(d.created_at.get(..16).unwrap_or("").replace('T', " "))
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
                    let (name, id) = (name.clone(), id.clone());
                    run(
                        &ctx,
                        done.clone(),
                        move |hub| hub.rollback(&name, &id),
                        window,
                        cx,
                    );
                }),
        )
}

#[cfg(test)]
mod tests {
    use super::rollback_targets;
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
        let ids: Vec<_> = rollback_targets(&web, list).into_iter().map(|d| d.id).collect();
        assert_eq!(ids, ["d2"]);
    }
}
