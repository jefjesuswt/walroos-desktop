mod api;
mod hub;
mod link;
mod node_card;
mod nodes;
mod page;
mod projects;
mod shell;
mod status;

use gpui_kit::component::theme::Theme;
use gpui_kit::*;

use link::{Link, Target};
use shell::Shell;

fn main() {
    let link = Link::new(Target::from_env());
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            let options = WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("walroos".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1000.), px(720.)),
                    cx,
                ))),
                app_id: Some("walroos-desktop".into()),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                follow_system_theme(window, cx);
                cx.new(|cx| Shell::new(link, cx))
            })
            .expect("open the window");
        });
}

/// Light or dark like the system, with walroos' magenta as the primary color. Changing the
/// mode reloads the theme's colors, so the brand goes back on after every change.
fn follow_system_theme(window: &mut Window, cx: &mut App) {
    Theme::sync_system_appearance(Some(window), cx);
    brand(cx);
    window
        .observe_window_appearance(|window, cx| {
            Theme::sync_system_appearance(Some(window), cx);
            brand(cx);
        })
        .detach();
}

/// The walroos primary: magenta, AA-contrast with its text in both modes (AGENTS.md, Brand).
fn brand(cx: &mut App) {
    Theme::update(cx, |t| {
        let (primary, on_primary): (Hsla, Hsla) = if t.is_dark() {
            (rgb(0xEE5A96).into(), rgb(0x1A0510).into())
        } else {
            (rgb(0xC21F65).into(), rgb(0xFFFFFF).into())
        };
        t.primary = primary;
        t.primary_hover = primary.opacity(0.9);
        t.primary_active = primary.opacity(0.8);
        t.primary_foreground = on_primary;
        t.button_primary = t.primary;
        t.button_primary_hover = t.primary_hover;
        t.button_primary_active = t.primary_active;
        t.button_primary_foreground = on_primary;
        t.progress_bar = primary;
        t.ring = primary;
    });
}
