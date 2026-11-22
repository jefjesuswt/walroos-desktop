//! The Nodes screen: every node of the fleet with its health, the ones that need you first.

use std::sync::Arc;
use std::time::Duration;

use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::Button;
use gpui_kit::component::label::Label;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme, StyledExt, h_flex, v_flex};
use gpui_kit::*;

use crate::api::{Node, User, sort_nodes};
use crate::hub::{self, Error, Hub};
use crate::node_card::node_card;

const REFRESH_EVERY: Duration = Duration::from_secs(10);

/// Where to find the hub: WALROOS_HUB and WALROOS_PROXY, as `walroos ui` takes --hub and --proxy.
#[derive(Clone)]
pub struct Target {
    pub hub: String,
    pub proxy: Option<String>,
}

impl Target {
    pub fn from_env() -> Target {
        let var = |k| std::env::var(k).ok().filter(|v: &String| !v.is_empty());
        Target {
            hub: var("WALROOS_HUB").unwrap_or_else(|| "walroos-hub".into()),
            proxy: var("WALROOS_PROXY"),
        }
    }
}

/// What one refresh brought back: the hub it went through and what it said.
struct Snapshot {
    hub: Arc<Hub>,
    me: User,
    nodes: Vec<Node>,
}

/// Runs on a background thread: reaches the hub if there's no connection yet, then lists the nodes.
fn load(target: Target, conn: Option<(Arc<Hub>, User)>) -> Result<Snapshot, Error> {
    let (hub, me) = match conn {
        Some(c) => c,
        None => {
            let (h, me) = hub::connect(&target.hub, target.proxy.as_deref())?;
            (Arc::new(h), me)
        }
    };
    let mut nodes = hub.nodes()?;
    sort_nodes(&mut nodes);
    Ok(Snapshot { hub, me, nodes })
}

pub struct NodesView {
    target: Target,
    conn: Option<(Arc<Hub>, User)>,
    nodes: Option<Vec<Node>>, // None until the first answer
    error: Option<String>,
    loading: bool,
    _poll: Task<()>,
}

impl NodesView {
    pub fn new(target: Target, cx: &mut Context<Self>) -> Self {
        let poll = cx.spawn(async move |this, cx| {
            loop {
                if this.update(cx, |v, cx| v.refresh(cx)).is_err() {
                    break; // the view is gone
                }
                cx.background_executor().timer(REFRESH_EVERY).await;
            }
        });
        NodesView {
            target,
            conn: None,
            nodes: None,
            error: None,
            loading: false,
            _poll: poll,
        }
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();
        let (target, conn) = (self.target.clone(), self.conn.clone());
        let task = cx.background_spawn(async move { load(target, conn) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |v, cx| v.apply(result, cx)).ok();
        })
        .detach();
    }

    fn apply(&mut self, result: Result<Snapshot, Error>, cx: &mut Context<Self>) {
        self.loading = false;
        match result {
            Ok(s) => {
                self.conn = Some((s.hub, s.me));
                self.nodes = Some(s.nodes);
                self.error = None;
            }
            Err(e) => {
                if matches!(e, Error::Unreachable(_)) {
                    self.conn = None; // reach it again next time: the way to it may have changed
                }
                self.error = Some(e.to_string());
            }
        }
        cx.notify();
    }

    fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let who = match &self.conn {
            Some((h, me)) => match &h.proxy {
                Some(p) => format!("{} as {}, through tailscaled's proxy at {p}", h.url, me.login),
                None => format!("{} as {}", h.url, me.login),
            },
            None => format!("Reaching {}…", hub::hub_url(&self.target.hub)),
        };
        h_flex()
            .justify_between()
            .items_center()
            .gap_4()
            .child(
                v_flex()
                    .child(Label::new("Nodes").text_xl().font_semibold())
                    .child(Label::new(who).text_sm().text_color(cx.theme().muted_foreground)),
            )
            .child(
                Button::new("refresh")
                    .outline()
                    .label("Refresh")
                    .loading(self.loading)
                    .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
            )
    }

    fn body(&self, cx: &mut Context<Self>) -> AnyElement {
        match &self.nodes {
            None if self.error.is_none() => h_flex()
                .gap_2()
                .child(Spinner::new())
                .child(Label::new("Loading nodes…"))
                .into_any_element(),
            None => div().into_any_element(),
            Some(nodes) if nodes.is_empty() => Label::new("No nodes yet. Join one with `walroos node join`.")
                .text_color(cx.theme().muted_foreground)
                .into_any_element(),
            Some(nodes) => v_flex()
                .gap_3()
                .children(nodes.iter().map(|n| node_card(n, cx)))
                .into_any_element(),
        }
    }
}

impl Render for NodesView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let error = self
            .error
            .clone()
            .map(|e| Alert::error("hub-error", e).title("Can't get the nodes from the hub"));
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .p_6()
            .gap_4()
            .child(self.header(cx))
            .children(error)
            .child(
                div()
                    .id("nodes")
                    .flex_1()
                    .overflow_y_scrollbar()
                    .child(self.body(cx)),
            )
    }
}
