//! One app's page: its header and actions, and a tab per part of its setup, like the web panel's
//! AppPage. It refreshes every few seconds, faster while a deploy runs.

mod automation;
mod dialogs;
mod environment;
mod header;
mod refs;
mod source;
mod summary;
mod tabs;

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::component::alert::Alert;
use gpui_kit::component::label::Label;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{ActiveTheme, h_flex, v_flex};
use gpui_kit::*;

use automation::Automation;
use source::{Done, SourceForm};

use crate::api::{App as AppService, Deployment, Ref};
use crate::env_editor::EnvEditor;
use crate::hub::{Error, Hub};
use crate::link::Link;
use crate::log_view::{LogView, Source};
use crate::page::{Ctx, poll};

const TABS: [&str; 7] = [
    "General",
    "Environment",
    "Domains",
    "Automation",
    "Deployments",
    "Logs",
    "Advanced",
];
const LOGS: usize = 5;

struct Data {
    app: AppService,
    deps: Vec<Deployment>,
}

fn fetch(hub: &Hub, name: &str) -> Result<Data, Error> {
    Ok(Data {
        app: hub.app(name)?,
        deps: hub.deployments(name)?,
    })
}

pub struct AppPage {
    name: String,
    link: Link,
    data: Option<Data>,
    error: Option<String>,
    busy: bool,
    tab: usize,
    editing: bool,
    form: Option<Entity<SourceForm>>,
    env: Option<Entity<EnvEditor>>,
    automation: Option<Entity<Automation>>,
    logs: Option<Entity<LogView>>,
    refs: Option<Vec<Ref>>, // what the app's variables can reference, once the tab asked
    _subs: Vec<Subscription>,
    _poll: Task<()>,
}

impl AppPage {
    pub fn new(name: String, link: Link, _: &mut Window, cx: &mut Context<Self>) -> Self {
        AppPage {
            name,
            link,
            data: None,
            error: None,
            busy: false,
            tab: 0,
            editing: false,
            form: None,
            env: None,
            automation: None,
            logs: None,
            refs: None,
            _subs: Vec::new(),
            _poll: poll(cx, |p, cx| {
                p.reload(cx);
                p.wait()
            }),
        }
    }

    fn wait(&self) -> Duration {
        let settling = self
            .data
            .as_ref()
            .and_then(|d| d.deps.first())
            .is_some_and(|d| d.state == "queued" || d.state == "running");
        Duration::from_secs(if settling { 2 } else { 8 })
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        let (link, name) = (self.link.clone(), self.name.clone());
        let task = cx.background_spawn(async move { link.call(|h| fetch(h, &name)) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |p, cx| p.apply(result, cx)).ok();
        })
        .detach();
    }

    fn apply(&mut self, result: Result<Data, Error>, cx: &mut Context<Self>) {
        self.busy = false;
        match result {
            Ok(d) => {
                if self.data.is_none() && d.app.is_draft() {
                    self.editing = true; // a draft shows its form right away
                }
                if let Some(a) = &self.automation {
                    a.update(cx, |a, cx| a.set_app(d.app.clone(), cx));
                }
                self.data = Some(d);
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        cx.notify();
    }

    fn ctx(&self, cx: &Context<Self>) -> Ctx {
        let page = cx.weak_entity();
        Ctx {
            link: self.link.clone(),
            manages_apps: self.link.me().is_some_and(|me| me.manages_apps()),
            refresh: Rc::new(move |cx| {
                page.update(cx, |p, cx| p.reload(cx)).ok();
            }),
        }
    }

    fn general(
        &mut self,
        a: &AppService,
        ctx: &Ctx,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if !self.editing {
            self.form = None;
            let page = cx.weak_entity();
            let edit = move |_: &mut Window, cx: &mut App| {
                page.update(cx, |p, cx| {
                    p.editing = true;
                    cx.notify();
                })
                .ok();
            };
            return summary::summary(a, ctx.manages_apps, edit, cx);
        }
        if !ctx.manages_apps {
            return summary::no_source(cx);
        }
        let form = match &self.form {
            Some(f) => f.clone(),
            None => {
                let f = cx.new(|cx| SourceForm::new(a, ctx.clone(), window, cx));
                self._subs.push(cx.subscribe(&f, |p, _, _: &Done, cx| {
                    p.editing = false;
                    p.reload(cx);
                }));
                self.form = Some(f.clone());
                f
            }
        };
        form.into_any_element()
    }

    fn tab_body(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(d) = &self.data else {
            return h_flex()
                .gap_2()
                .child(Spinner::new())
                .child(Label::new("Loading…"))
                .into_any_element();
        };
        let (a, deps, ctx) = (d.app.clone(), d.deps.clone(), self.ctx(cx));
        match self.tab {
            0 => self.general(&a, &ctx, window, cx),
            1 => self.environment(&a, &ctx, window, cx),
            2 => tabs::domains(&a, &ctx, cx),
            3 => self
                .automation
                .get_or_insert_with(|| cx.new(|cx| Automation::new(a.clone(), ctx.clone(), cx)))
                .clone()
                .into_any_element(),
            4 => tabs::deployments(&a, &deps, &ctx, cx),
            LOGS => self
                .logs
                .get_or_insert_with(|| {
                    cx.new(|cx| {
                        LogView::embedded(Source::Runtime { app: a.name.clone() }, ctx.link.clone(), cx)
                    })
                })
                .clone()
                .into_any_element(),
            _ => tabs::advanced(&a, &ctx, cx),
        }
    }
}

impl Render for AppPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let error = self
            .error
            .clone()
            .map(|e| Alert::error("app-error", e).title("Can't get the app from the hub"));
        let header = self.data.as_ref().map(|d| d.app.clone()).map(|a| {
            let ctx = self.ctx(cx);
            let down = a.node_down.then(|| {
                Alert::warning("node-down", "Deploys and logs wait for it. The app keeps running if the node is up but the hub can't reach it.")
                    .title(format!("{} isn't answering", a.node_name))
            });
            v_flex().gap_3().child(self.header(&a, &ctx, cx)).children(down)
        });
        let tabs = TabBar::new("app-tabs")
            .underline()
            .selected_index(self.tab)
            .children(TABS.map(|t| Tab::new().label(t)))
            .on_click(cx.listener(|this, ix: &usize, _, cx| {
                this.tab = *ix;
                cx.notify();
            }));
        let body = self.tab_body(window, cx);
        // The Logs tab scrolls inside its own box; the others scroll as a page.
        let body = if self.tab == LOGS {
            div().flex_1().min_h_0().child(body)
        } else {
            div().flex_1().min_h_0().child(
                div()
                    .id("app-body")
                    .size_full()
                    .overflow_y_scrollbar()
                    .child(v_flex().max_w(px(860.)).pb_6().child(body)),
            )
        };
        v_flex()
            .size_full()
            .px_6()
            .pt_2()
            .gap_4()
            .bg(cx.theme().background)
            .children(header)
            .children(error)
            .child(tabs)
            .child(body)
    }
}
