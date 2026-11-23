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
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::*;

pub use this_app::Switched;
use this_app::ThisApp;

use crate::api::{Cloudflare, GitHub, Member, SshKey, User};
use crate::hub::{Error, Hub};
use crate::link::Link;
use crate::page::{Ctx, frame, heading, scroll_body};

/// From this window width the cards go in two columns.
const WIDE: f32 = 1100.;

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

    /// The cards in two columns: this app, the people and your keys; then the integrations.
    fn columns(&self, cx: &Context<Self>) -> (Vec<AnyElement>, Vec<AnyElement>) {
        let mut left = vec![self.this_app.clone().into_any_element()];
        let Some(Ok(d)) = &self.data else {
            left.push(self.waiting());
            return (left, vec![]);
        };
        let ctx = self.ctx(&d.me, cx);
        let mut right = Vec::new();
        if let Some(o) = &d.owned {
            left.push(users::card(&o.users, &d.me, &ctx, cx));
            right.push(tokens::cloudflare(&o.cloudflare, &self.cf_token, &ctx, cx));
            right.push(tokens::github(&o.github, &self.gh_token, &ctx, cx));
        }
        right.push(keys::card(&d.keys, &ctx, cx));
        (left, right)
    }

    /// Instead of the hub's cards: loading, or why they can't show.
    fn waiting(&self) -> AnyElement {
        match &self.data {
            Some(Err(e)) => Alert::error("settings-error", e.clone())
                .title("Can't get your settings from the hub")
                .into_any_element(),
            _ => h_flex()
                .gap_2()
                .child(Spinner::new())
                .child(Label::new("Loading…"))
                .into_any_element(),
        }
    }
}

impl Render for Settings {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (left, right) = self.columns(cx);
        let column = |cards: Vec<AnyElement>| v_flex().flex_1().min_w_0().gap_4().children(cards);
        let body = if window.viewport_size().width >= px(WIDE) {
            h_flex()
                .gap_4()
                .items_start()
                .child(column(left))
                .child(column(right))
        } else {
            v_flex().w_full().gap_4().child(column(left)).child(column(right))
        };
        frame()
            .child(heading("Settings", self.link.describe(), cx))
            .child(scroll_body("settings-body", body))
    }
}
