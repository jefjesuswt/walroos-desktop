//! Which hub this app reaches and through which proxy, with the hubs found on your tailnet to
//! pick from. It reaches the hub with the new values first and only saves them when the hub
//! answers, so a typo never leaves the app pointed nowhere.

use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::form::{field, h_form};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::label::Label;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme, Disableable, Sizable, h_flex, v_flex};
use gpui_kit::*;

use crate::api::User;
use crate::config;
use crate::discover;
use crate::hub::{self, DEFAULT_PROXY, Error, Hub};
use crate::link::{Link, Target};
use crate::ui::section;

/// The app switched to another hub: what showed the old one starts over.
pub struct Switched;

/// A hub found on the tailnet: its name and your role there, or why it won't have you.
type Found = (String, Result<String, String>);

pub struct ThisApp {
    link: Link,
    hub: Entity<InputState>,
    proxy: Entity<InputState>,
    checking: bool,
    outcome: Option<Result<String, String>>,
    found: Option<Result<Vec<Found>, String>>, // None while searching
    _enter: [Subscription; 2],
}

impl EventEmitter<Switched> for ThisApp {}

impl ThisApp {
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
        let mut this = ThisApp {
            _enter: [enter(cx, &hub), enter(cx, &proxy)],
            link,
            hub,
            proxy,
            checking: false,
            outcome: None,
            found: None,
        };
        this.search(cx);
        this
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

    fn search(&mut self, cx: &mut Context<Self>) {
        self.found = None;
        cx.notify();
        let proxy = self.typed(cx).proxy;
        let task = cx.background_spawn(async move {
            discover::hubs(proxy.as_deref()).map(|list| {
                list.into_iter()
                    .map(|f| (f.name, f.me.map(|me| me.role)))
                    .collect()
            })
        });
        cx.spawn(async move |this, cx| {
            let found = task.await;
            this.update(cx, |s, cx| {
                s.found = Some(found);
                cx.notify();
            })
            .ok();
        })
        .detach();
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

    fn pick(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let name = name.to_string();
        self.hub.update(cx, |s, cx| s.set_value(name, window, cx));
        self.connect(cx);
    }

    fn note(&self) -> Option<Alert> {
        let vars = config::overridden();
        (!vars.is_empty()).then(|| {
            let text = format!(
                "{} is set, so the next start uses it over what you save here.",
                vars.join(" and ")
            );
            Alert::warning("env-override", text)
        })
    }

    fn found(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let list = match &self.found {
            None => {
                return h_flex()
                    .gap_2()
                    .child(Spinner::new().small())
                    .child(
                        Label::new("Looking for hubs on your tailnet…")
                            .text_sm()
                            .text_color(muted),
                    )
                    .into_any_element();
            }
            Some(Err(e)) => {
                return Label::new(e.clone())
                    .text_sm()
                    .text_color(muted)
                    .into_any_element();
            }
            Some(Ok(list)) if list.is_empty() => {
                let text = "No hub answered on your tailnet. Type its name.";
                return Label::new(text).text_sm().text_color(muted).into_any_element();
            }
            Some(Ok(list)) => list,
        };
        let current = self.link.target().hub;
        v_flex()
            .gap_1()
            .children(list.iter().map(|(name, me)| {
                let (n, said) = (name.clone(), me.as_ref().map(|r| format!("you're {r}")));
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(Label::new(name.clone()).font_family(cx.theme().mono_font_family.clone()))
                    .child(
                        Label::new(said.unwrap_or_else(|e| e.clone()))
                            .text_sm()
                            .text_color(muted),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new(SharedString::from(format!("use-{name}")))
                            .small()
                            .outline()
                            .label(if *name == current { "In use" } else { "Use" })
                            .disabled(me.is_err() || *name == current || self.checking)
                            .on_click(cx.listener(move |this, _, window, cx| this.pick(&n, window, cx))),
                    )
            }))
            .into_any_element()
    }
}

impl Render for ThisApp {
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
        let search = Button::new("search")
            .small()
            .ghost()
            .label("Search again")
            .disabled(self.found.is_none())
            .on_click(cx.listener(|this, _, _, cx| this.search(cx)));
        let save = Button::new("connect")
            .primary()
            .label("Connect and save")
            .loading(self.checking)
            .on_click(cx.listener(|this, _, _, cx| this.connect(cx)));
        let sub = config::path()
            .map(|p| format!("Which hub this app reaches. Kept in {}", p.display()))
            .unwrap_or_else(|| "Which hub this app reaches.".into());
        section("This app", sub, Some(search.into_any_element()), cx)
            .children(self.note())
            .child(self.found(cx))
            .child(form)
            .children(outcome)
            .child(h_flex().justify_end().child(save))
    }
}
