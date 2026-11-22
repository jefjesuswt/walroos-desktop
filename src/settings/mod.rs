//! The Settings tab, like the web panel's: which hub this app reaches, and on the hub your SSH
//! keys and, for owners, its users and the Cloudflare and GitHub tokens it keeps.

mod keys;
mod this_app;
mod tokens;
mod users;

use std::rc::Rc;

use gpui_kit::component::alert::Alert;
use gpui_kit::component::input::InputState;
use gpui_kit::component::label::Label;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme, StyledExt, h_flex, v_flex};
use gpui_kit::*;

pub use this_app::Switched;
use this_app::ThisApp;

use crate::api::{Cloudflare, GitHub, Member, SshKey, User};
use crate::hub::{Error, Hub};
use crate::link::Link;
use crate::page::{Ctx, card, heading};

/// What Settings shows from the hub.
struct HubData {
    me: User,
    keys: Vec<SshKey>,
    owned: Option<Owned>, // the owner's part
}

struct Owned {
    users: Vec<Member>,
    cloudflare: Cloudflare,
    github: GitHub,
}

fn fetch(hub: &Hub) -> Result<HubData, Error> {
    let me = hub.me()?;
    let owned = match me.owns_hub() {
        true => Some(Owned {
            users: hub.users()?,
            cloudflare: hub.cloudflare()?,
            github: hub.github()?,
        }),
        false => None,
    };
    Ok(HubData {
        keys: hub.ssh_keys()?,
        me,
        owned,
    })
}

pub struct Settings {
    link: Link,
    this_app: Entity<ThisApp>,
    data: Option<Result<HubData, String>>, // None until the first answer
    cf_token: Entity<InputState>,
    gh_token: Entity<InputState>,
    _switched: Subscription,
}

impl EventEmitter<Switched> for Settings {}

impl Settings {
    pub fn new(link: Link, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let this_app = cx.new(|cx| ThisApp::new(link.clone(), window, cx));
        let switched = cx.subscribe(&this_app, |this, _, _: &Switched, cx| {
            this.reload(cx);
            cx.emit(Switched);
        });
        let token = |window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| InputState::new(window, cx).masked(true))
        };
        let mut this = Settings {
            cf_token: token(window, cx),
            gh_token: token(window, cx),
            link,
            this_app,
            data: None,
            _switched: switched,
        };
        this.reload(cx);
        this
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        let link = self.link.clone();
        let task = cx.background_spawn(async move { link.call(fetch) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |s, cx| {
                s.data = Some(result.map_err(|e| e.to_string()));
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn ctx(&self, me: &User, cx: &Context<Self>) -> Ctx {
        let page = cx.weak_entity();
        Ctx {
            link: self.link.clone(),
            manages_apps: me.manages_apps(),
            refresh: Rc::new(move |cx| {
                page.update(cx, |s, cx| s.reload(cx)).ok();
            }),
        }
    }

    fn hub_sections(&self, cx: &Context<Self>) -> Vec<AnyElement> {
        let d = match &self.data {
            None => {
                let loading = h_flex()
                    .gap_2()
                    .child(Spinner::new())
                    .child(Label::new("Loading…"));
                return vec![loading.into_any_element()];
            }
            Some(Err(e)) => {
                let alert =
                    Alert::error("settings-error", e.clone()).title("Can't get your settings from the hub");
                return vec![alert.into_any_element()];
            }
            Some(Ok(d)) => d,
        };
        let ctx = self.ctx(&d.me, cx);
        let mut out = Vec::new();
        if let Some(o) = &d.owned {
            out.push(users::card(&o.users, &d.me, &ctx, cx));
            out.push(tokens::cloudflare(&o.cloudflare, &self.cf_token, &ctx, cx));
            out.push(tokens::github(&o.github, &self.gh_token, &ctx, cx));
        }
        out.push(keys::card(&d.keys, &ctx, cx));
        out
    }
}

impl Render for Settings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let column = v_flex()
            .gap_4()
            .max_w(px(720.))
            .pb_6()
            .child(self.this_app.clone())
            .children(self.hub_sections(cx));
        v_flex()
            .size_full()
            .p_6()
            .gap_4()
            .child(heading("Settings", self.link.describe(), cx))
            .child(
                div()
                    .id("settings-body")
                    .flex_1()
                    .overflow_y_scrollbar()
                    .child(column),
            )
    }
}

/// A settings card: its title and what it's for, an action on the right, then its content.
fn section(title: &'static str, sub: impl Into<SharedString>, action: Option<AnyElement>, cx: &App) -> Div {
    let head = h_flex()
        .gap_4()
        .items_start()
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(Label::new(title).font_semibold())
                .child(Label::new(sub).text_sm().text_color(cx.theme().muted_foreground)),
        )
        .children(action);
    card(cx).child(head)
}

/// A muted line under a row's main text.
fn muted(text: impl Into<SharedString>, cx: &App) -> Label {
    Label::new(text).text_sm().text_color(cx.theme().muted_foreground)
}
