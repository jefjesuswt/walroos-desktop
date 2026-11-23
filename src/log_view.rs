//! A log in a side sheet: a deployment's build log, live while it runs, or what the app's
//! running release writes. It follows the newest line.

use std::time::Duration;

use gpui_kit::component::alert::Alert;
use gpui_kit::component::label::Label;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{ActiveTheme, Sizable, WindowExt, h_flex, v_flex};
use gpui_kit::*;

use crate::api::Deployment;
use crate::link::Link;
use crate::status::status_tag;
use crate::when;

/// What the sheet shows.
#[derive(Clone)]
pub enum Source {
    Deploy { app: String, id: String },
    Runtime { app: String },
}

enum Loaded {
    Deploy(Deployment),
    Runtime(String),
}

const RUNTIME_LINES: u32 = 300;
/// The sheet's own title bar and padding, above and below the view.
const SHEET_CHROME: f32 = 110.;

pub struct LogView {
    source: Source,
    link: Link,
    deployment: Option<Deployment>,
    lines: Option<Vec<String>>, // None until the first answer
    error: Option<String>,
    scroll: ScrollHandle,
    follow: bool,        // new lines came in: go to the bottom once they're laid out
    chrome: Option<f32>, // in a sheet: the window's height the log box doesn't get
    _poll: Task<()>,
}

/// Opens the log of source in a sheet on the right.
pub fn open(source: Source, link: Link, window: &mut Window, cx: &mut App) {
    let title = match &source {
        Source::Deploy { app, .. } => format!("{app} · deploy"),
        Source::Runtime { app } => format!("{app} · logs"),
    };
    let view = cx.new(|cx| LogView::new(source, link, Some(SHEET_CHROME), cx));
    window.open_sheet(cx, move |sheet, _, _| {
        sheet.title(title.clone()).size(px(760.)).child(view.clone())
    });
}

impl LogView {
    /// The log in a page (the app's Logs tab) instead of a sheet.
    pub fn embedded(source: Source, link: Link, cx: &mut Context<Self>) -> Self {
        LogView::new(source, link, None, cx)
    }

    fn new(source: Source, link: Link, chrome: Option<f32>, cx: &mut Context<Self>) -> Self {
        let poll = cx.spawn(async move |this, cx| {
            loop {
                let Ok(task) = this.update(cx, |v, cx| v.fetch(cx)) else {
                    break;
                };
                let result = task.await;
                let Ok(next) = this.update(cx, |v, cx| v.apply(result, cx)) else {
                    break;
                };
                let Some(wait) = next else { break }; // a finished deployment's log doesn't change
                cx.background_executor().timer(wait).await;
            }
        });
        LogView {
            source,
            link,
            deployment: None,
            lines: None,
            error: None,
            scroll: ScrollHandle::new(),
            follow: false,
            chrome,
            _poll: poll,
        }
    }

    fn fetch(&self, cx: &mut Context<Self>) -> Task<Result<Loaded, crate::hub::Error>> {
        let (link, source) = (self.link.clone(), self.source.clone());
        cx.background_spawn(async move {
            link.call(|hub| match &source {
                Source::Deploy { app, id } => hub.deployment(app, id).map(Loaded::Deploy),
                Source::Runtime { app } => hub.app_logs(app, RUNTIME_LINES).map(Loaded::Runtime),
            })
        })
    }

    /// Takes in an answer and says when to ask again, if ever.
    fn apply(
        &mut self,
        result: Result<Loaded, crate::hub::Error>,
        cx: &mut Context<Self>,
    ) -> Option<Duration> {
        let text = match result {
            Err(e) => {
                self.error = Some(e.to_string());
                cx.notify();
                return Some(Duration::from_secs(5));
            }
            Ok(Loaded::Runtime(text)) => text,
            Ok(Loaded::Deploy(d)) => {
                let text = d.log.clone();
                self.deployment = Some(d);
                text
            }
        };
        self.error = None;
        let lines: Vec<String> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(str::to_string)
            .collect();
        if self.lines.as_ref().map(Vec::len) != Some(lines.len()) {
            self.follow = true;
        }
        self.lines = Some(lines);
        cx.notify();
        match (&self.source, &self.deployment) {
            (Source::Runtime { .. }, _) => Some(Duration::from_secs(5)),
            (Source::Deploy { .. }, Some(d)) if d.state == "queued" || d.state == "running" => {
                Some(Duration::from_millis(1500))
            }
            _ => None,
        }
    }

    fn summary(&self, cx: &App) -> Option<impl IntoElement> {
        let d = self.deployment.as_ref()?;
        let mut facts = vec![format!("started {}", when::short(&d.created_at))];
        if let Some(took) = when::took(&d.created_at, d.finished_at.as_deref()) {
            facts.push(format!(
                "{} {took}",
                if d.finished_at.is_some() {
                    "took"
                } else {
                    "running for"
                }
            ));
        }
        if !d.sha.is_empty() {
            facts.insert(0, d.sha.chars().take(7).collect());
        }
        Some(
            h_flex()
                .gap_2()
                .items_center()
                .child(status_tag(&d.state, d.state.clone()))
                .child(
                    Label::new(facts.join(" · "))
                        .text_sm()
                        .text_color(cx.theme().muted_foreground),
                ),
        )
    }

    fn empty_text(&self) -> &'static str {
        match (&self.lines, &self.deployment) {
            (None, _) => "Loading…",
            (_, Some(d)) if d.state == "queued" => "Waiting for the deploy before it to finish…",
            _ => "No output yet.",
        }
    }
}

/// How a line reads: an error, a step of the deploy, or detail.
#[derive(Debug, PartialEq)]
pub enum LineKind {
    Error,
    Step,
    Detail,
}

/// The web panel's rule (DeploymentModal.tsx), so a log reads the same in both.
pub fn line_kind(line: &str) -> LineKind {
    let l = line.to_lowercase();
    if l.contains("never got healthy")
        || l.starts_with("error")
        || l.contains("failed")
        || l.contains("fatal")
    {
        return LineKind::Error;
    }
    let steps = [
        "cloning",
        "building",
        "starting",
        "health check",
        "stopping",
        "rollback",
    ];
    if steps.iter().any(|s| line.starts_with(s)) || line.contains("is live") {
        LineKind::Step
    } else {
        LineKind::Detail
    }
}

impl Render for LogView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if std::mem::take(&mut self.follow) {
            cx.on_next_frame(window, |this, _, _| this.scroll.scroll_to_bottom());
        }

        let theme = cx.theme();
        let failed = self
            .deployment
            .as_ref()
            .filter(|d| d.state == "failed")
            .and_then(|d| d.error.clone());
        let lines = self.lines.as_deref().unwrap_or_default();
        let body = if lines.is_empty() {
            vec![
                Label::new(self.empty_text())
                    .text_color(theme.muted_foreground)
                    .into_any_element(),
            ]
        } else {
            lines
                .iter()
                .map(|l| {
                    let color = match line_kind(l) {
                        LineKind::Error => theme.danger,
                        LineKind::Step => theme.foreground,
                        LineKind::Detail => theme.muted_foreground,
                    };
                    div().text_color(color).child(l.clone()).into_any_element()
                })
                .collect()
        };
        // ponytail: the sheet puts its content in a scroll area, where a full height doesn't bound
        // it; sizing to the window (less what's around the log) keeps the log box scrolling inside
        // itself, so it can follow the newest line.
        let viewport = window.viewport_size().height;
        let box_ = match self.chrome {
            Some(chrome) => v_flex().w_full().h(viewport - px(chrome)),
            None => v_flex().size_full(), // a page bounds it
        };
        box_.gap_3()
            .children(self.summary(cx))
            .children(
                self.error
                    .clone()
                    .map(|e| Alert::error("log-error", e).title("Can't get the log").small()),
            )
            .children(failed.map(|e| Alert::error("deploy-error", e).title("The deploy failed").small()))
            .child(
                div()
                    .id("log")
                    .flex_1()
                    .min_h_0() // fill what's left of the sheet and scroll inside, not grow the sheet
                    .rounded(theme.radius)
                    .bg(theme.muted)
                    .font_family(theme.mono_font_family.clone())
                    .text_size(theme.mono_font_size)
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .vertical_scrollbar(&self.scroll)
                    // padding inside: on the scrolling box gpui counts it as overflow
                    .child(div().p_3().children(body)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{LineKind, line_kind};

    #[test]
    fn reads_lines_like_the_web_panel() {
        assert_eq!(line_kind("building image registry/web:abc"), LineKind::Step);
        assert_eq!(line_kind("web-d2 is live"), LineKind::Step);
        assert_eq!(line_kind("Error: exit status 1"), LineKind::Error);
        assert_eq!(line_kind("web-d2 never got healthy on /healthz"), LineKind::Error);
        assert_eq!(line_kind("#5 [2/4] RUN bun install"), LineKind::Detail);
    }
}
