mod actions;
mod api;
mod app_page;
mod command;
mod config;
mod deployments;
mod discover;
mod env_editor;
mod hub;
mod link;
mod log_view;
mod nav;
mod node_card;
mod nodes;
mod page;
mod projects;
mod rollback;
mod settings;
mod shell;
mod status;
mod theme;
mod ui;
mod when;
mod workflow;

use gpui_kit::*;

use link::Link;
use shell::Shell;

fn main() {
    let link = Link::new(config::load());
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
                theme::follow_system(window, cx);
                cx.new(|cx| Shell::new(link, cx))
            })
            .expect("open the window");
        });
}
