//! What you can do to an app from its row's ⋮ menu: read its logs, and, if your role manages
//! apps, deploy, start, stop, restart and roll back. A change runs on the hub in the background,
//! says how it went in a notification and refreshes the page; a deploy opens its log.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_kit::component::menu::{DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{Icon, Sizable, WindowExt};
use gpui_kit::*;

use crate::api::{App as AppService, Deployment};
use crate::hub::{Error, Hub};
use crate::log_view::{self, Source};
use crate::page::Ctx;
use crate::rollback;

/// Runs change on the hub off the UI thread; then tells you `done` (or what went wrong),
/// refreshes the page and hands the answer to then.
pub fn run<R: Send + 'static>(
    ctx: &Ctx,
    done: String,
    change: impl FnOnce(&Hub) -> Result<R, Error> + Send + 'static,
    then: impl FnOnce(R, &mut Window, &mut App) + 'static,
    window: &mut Window,
    cx: &mut App,
) {
    let (link, refresh, handle) = (ctx.link.clone(), ctx.refresh.clone(), window.window_handle());
    let task = cx.background_spawn(async move { link.call(change) });
    cx.spawn(async move |cx| {
        let result = task.await;
        cx.update_window(handle, |_, window, cx| {
            match result {
                Ok(answer) => {
                    window.push_notification(Notification::success(done), cx);
                    then(answer, window, cx);
                }
                Err(e) => window.push_notification(Notification::error(e.to_string()), cx),
            }
            refresh(cx);
        })
        .ok();
    })
    .detach();
}

/// After a deploy or a rollback starts: its log, live.
pub fn follow(ctx: &Ctx, app: &str) -> impl FnOnce(Deployment, &mut Window, &mut App) + 'static {
    let (link, app) = (ctx.link.clone(), app.to_string());
    move |d, window, cx| log_view::open(Source::Deploy { app, id: d.id }, link, window, cx)
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
    if a.running.is_some() && !a.node_down {
        menu = menu.item(item("Logs", a, ctx, |a, ctx, w, cx| {
            log_view::open(Source::Runtime { app: a.name.clone() }, ctx.link.clone(), w, cx)
        }));
    }
    if let Some(d) = &a.last_deployment {
        let id = d.id.clone();
        let (app, link) = (a.name.clone(), ctx.link.clone());
        menu = menu.item(PopupMenuItem::new("Last deploy log").on_click(move |_, w, cx| {
            log_view::open(
                Source::Deploy {
                    app: app.clone(),
                    id: id.clone(),
                },
                link.clone(),
                w,
                cx,
            )
        }));
    }
    if ctx.manages_apps {
        menu = change_items(menu.separator(), a, ctx);
    }
    menu
}

fn change_items(mut menu: PopupMenu, a: &AppService, ctx: &Ctx) -> PopupMenu {
    let has_source = !a.repo_url.is_empty() || !a.image.is_empty();
    let deploy = if a.running.is_some() { "Redeploy" } else { "Deploy" };
    menu = menu.item(item(deploy, a, ctx, deploy_app).disabled(!has_source));
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
        menu = menu.separator().item(item("Roll back…", a, ctx, rollback::open));
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

fn deploy_app(a: &AppService, ctx: &Ctx, window: &mut Window, cx: &mut App) {
    let name = a.name.clone();
    let done = format!("Deploying {name}");
    run(
        ctx,
        done,
        move |hub| hub.deploy(&name),
        follow(ctx, &a.name),
        window,
        cx,
    );
}

fn power(a: &AppService, ctx: &Ctx, action: &'static str, window: &mut Window, cx: &mut App) {
    let done = match action {
        "stop" => "stopped",
        "start" => "started",
        _ => "restarted",
    };
    let name = a.name.clone();
    let done = format!("{name} {done}");
    run(
        ctx,
        done,
        move |hub| hub.power(&name, action),
        |_, _, _| {},
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
