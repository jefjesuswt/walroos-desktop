//! A page that shows something the hub lists: it fetches it in the background, refreshes it
//! every few seconds, and keeps showing the last answer under an error when a refresh fails.

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::Button;
use gpui_kit::component::label::Label;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme, StyledExt, h_flex, v_flex};
use gpui_kit::*;

use crate::hub::{Error, Hub};
use crate::link::Link;

const REFRESH_EVERY: Duration = Duration::from_secs(10);
/// While something on the page is changing (a deploy running), it refreshes faster.
const REFRESH_BUSY: Duration = Duration::from_secs(2);

/// What a page is: its title, how to get its data and how to draw it.
pub struct Spec<T> {
    pub title: &'static str,
    pub fetch: fn(&Hub) -> Result<T, Error>,
    pub body: fn(&T, &Ctx, &App) -> AnyElement,
    /// Whether the data is in the middle of changing, so the page refreshes faster.
    pub settling: fn(&T) -> bool,
}

/// What a page's body needs to act on what it shows: the hub, whether you may change apps, and
/// a way to refresh the page after a change.
#[derive(Clone)]
pub struct Ctx {
    pub link: Link,
    pub manages_apps: bool,
    pub refresh: Rc<dyn Fn(&mut App)>,
}

pub struct Page<T: 'static> {
    spec: Spec<T>,
    link: Link,
    data: Option<T>, // None until the first answer
    error: Option<String>,
    busy: bool,     // a request is out
    spinning: bool, // you asked for it: the Refresh button shows it; polling stays quiet
    _poll: Task<()>,
}

impl<T: Send + 'static> Page<T> {
    pub fn new(spec: Spec<T>, link: Link, cx: &mut Context<Self>) -> Self {
        let poll = poll(cx, |p, cx| {
            p.refresh(cx);
            p.wait()
        });
        Page {
            spec,
            link,
            data: None,
            error: None,
            busy: false,
            spinning: false,
            _poll: poll,
        }
    }

    fn wait(&self) -> Duration {
        match &self.data {
            Some(data) if (self.spec.settling)(data) => REFRESH_BUSY,
            _ => REFRESH_EVERY,
        }
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        let (link, fetch) = (self.link.clone(), self.spec.fetch);
        let task = cx.background_spawn(async move { link.call(fetch) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |p, cx| p.apply(result, cx)).ok();
        })
        .detach();
    }

    fn apply(&mut self, result: Result<T, Error>, cx: &mut Context<Self>) {
        self.busy = false;
        self.spinning = false;
        match result {
            Ok(data) => {
                self.data = Some(data);
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        cx.notify();
    }

    fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .justify_between()
            .items_center()
            .gap_4()
            .child(heading(self.spec.title, self.link.describe(), cx))
            .child(
                Button::new("refresh")
                    .outline()
                    .label("Refresh")
                    .loading(self.spinning)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.spinning = true;
                        cx.notify();
                        this.refresh(cx);
                    })),
            )
    }

    fn ctx(&self, cx: &Context<Self>) -> Ctx {
        let page = cx.weak_entity();
        Ctx {
            link: self.link.clone(),
            manages_apps: self.link.me().is_some_and(|me| me.manages_apps()),
            refresh: Rc::new(move |cx| {
                page.update(cx, |p, cx| p.refresh(cx)).ok();
            }),
        }
    }

    fn body(&self, cx: &Context<Self>) -> AnyElement {
        match &self.data {
            Some(data) => (self.spec.body)(data, &self.ctx(cx), cx),
            None if self.error.is_none() => h_flex()
                .gap_2()
                .child(Spinner::new())
                .child(Label::new("Loading…"))
                .into_any_element(),
            None => div().into_any_element(),
        }
    }
}

impl<T: Send + 'static> Render for Page<T> {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let error = self.error.clone().map(|e| {
            Alert::error("hub-error", e).title(format!(
                "Can't get the {} from the hub",
                self.spec.title.to_lowercase()
            ))
        });
        v_flex()
            .size_full()
            .p_6()
            .gap_4()
            .child(self.header(cx))
            .children(error)
            .child(
                div()
                    .id("page-body")
                    .flex_1()
                    .overflow_y_scrollbar()
                    .child(self.body(cx)),
            )
    }
}

/// Calls tick now, then again after the wait it returns, for as long as the view lives.
pub fn poll<V: 'static>(cx: &mut Context<V>, tick: fn(&mut V, &mut Context<V>) -> Duration) -> Task<()> {
    cx.spawn(async move |this, cx| {
        loop {
            let Ok(wait) = this.update(cx, tick) else {
                break; // the view is gone
            };
            cx.background_executor().timer(wait).await;
        }
    })
}

/// A page's title, with a muted line under it.
pub fn heading(title: &'static str, under: String, cx: &App) -> Div {
    v_flex().child(Label::new(title).text_xl().font_semibold()).child(
        Label::new(under)
            .text_sm()
            .text_color(cx.theme().muted_foreground),
    )
}

/// A muted line of text for an empty list.
pub fn empty(text: &'static str, cx: &App) -> AnyElement {
    Label::new(text)
        .text_color(cx.theme().muted_foreground)
        .into_any_element()
}

/// The box every item of a list sits in.
pub fn card(cx: &App) -> Div {
    v_flex()
        .gap_3()
        .p_4()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius)
        .bg(cx.theme().group_box)
}
