//! The Settings tab: which hub to reach and through which proxy. It reaches the hub with the new
//! values first and only saves them when the hub answers, so a typo never leaves the app pointed
//! nowhere.

use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::form::{field, h_form};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::*;

use crate::api::User;
use crate::config;
use crate::hub::{self, DEFAULT_PROXY, Error, Hub};
use crate::link::{Link, Target};
use crate::page::{card, heading};

/// Settings switched the app to another hub: the pages showing the old one start over.
pub struct Switched;

pub struct Settings {
    link: Link,
    hub: Entity<InputState>,
    proxy: Entity<InputState>,
    checking: bool,
    outcome: Option<Result<String, String>>,
    _enter: [Subscription; 2],
}

impl EventEmitter<Switched> for Settings {}

impl Settings {
    pub fn new(link: Link, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let t = link.target();
        let hub = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(config::DEFAULT_HUB)
                .default_value(t.hub)
        });
        let proxy = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(DEFAULT_PROXY)
                .default_value(t.proxy.unwrap_or_default())
        });
        let enter = |cx: &mut Context<Self>, input: &Entity<InputState>| {
            cx.subscribe(input, |this, _, e: &InputEvent, cx| {
                if let InputEvent::PressEnter { .. } = e {
                    this.connect(cx);
                }
            })
        };
        Settings {
            _enter: [enter(cx, &hub), enter(cx, &proxy)],
            link,
            hub,
            proxy,
            checking: false,
            outcome: None,
        }
    }

    fn typed(&self, cx: &App) -> Target {
        let text = |i: &Entity<InputState>| i.read(cx).value().trim().to_string();
        let hub = text(&self.hub);
        Target {
            hub: if hub.is_empty() {
                config::DEFAULT_HUB.into()
            } else {
                hub
            },
            proxy: Some(text(&self.proxy)).filter(|p| !p.is_empty()),
        }
    }

    fn connect(&mut self, cx: &mut Context<Self>) {
        if self.checking {
            return;
        }
        self.checking = true;
        self.outcome = None;
        cx.notify();
        let target = self.typed(cx);
        let t = target.clone();
        let task = cx.background_spawn(async move { hub::connect(&t.hub, t.proxy.as_deref()) });
        cx.spawn(async move |this, cx| {
            let reached = task.await;
            this.update(cx, |s, cx| s.reached(target, reached, cx)).ok();
        })
        .detach();
    }

    fn reached(&mut self, target: Target, reached: Result<(Hub, User), Error>, cx: &mut Context<Self>) {
        self.checking = false;
        self.outcome = Some(match reached {
            Err(e) => Err(e.to_string()),
            Ok((hub, me)) => config::save(&target).map(|()| {
                let said = format!("Reached {} as {} ({}). Saved.", hub.url, me.login, me.role);
                self.link.switch(target, hub, me);
                cx.emit(Switched);
                said
            }),
        });
        cx.notify();
    }

    fn note(&self) -> Option<Alert> {
        let vars = config::overridden();
        (!vars.is_empty()).then(|| {
            let saved = config::path()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            Alert::warning(
                "env-override",
                format!(
                    "{} is set, so the next start uses it over what you save here ({saved}).",
                    vars.join(" and ")
                ),
            )
        })
    }
}

impl Render for Settings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let outcome = self.outcome.clone().map(|o| match o {
            Ok(m) => Alert::success("outcome", m),
            Err(m) => Alert::error("outcome", m).title("Can't reach that hub. Nothing was saved."),
        });
        let form = h_form()
            .label_width(px(72.))
            .child(
                field()
                    .label("Hub")
                    .description("Its name on the tailnet, or a URL.")
                    .child(Input::new(&self.hub)),
            )
            .child(
                field()
                    .label("Proxy")
                    .description(format!("Empty: direct, then tailscaled's at {DEFAULT_PROXY}."))
                    .child(Input::new(&self.proxy)),
            );
        let save = Button::new("connect")
            .primary()
            .label("Connect and save")
            .loading(self.checking)
            .on_click(cx.listener(|this, _, _, cx| this.connect(cx)));
        v_flex()
            .size_full()
            .p_6()
            .gap_4()
            .child(heading("Settings", saved_in(), cx))
            .children(self.note())
            .child(
                card(cx)
                    .max_w(px(640.))
                    .child(form)
                    .children(outcome)
                    .child(h_flex().justify_end().child(save)),
            )
    }
}

/// Where this app finds the hub, and the file it's kept in.
fn saved_in() -> String {
    match config::path() {
        Some(p) => format!("Which hub this app reaches. Kept in {}", p.display()),
        None => "Which hub this app reaches.".into(),
    }
}
