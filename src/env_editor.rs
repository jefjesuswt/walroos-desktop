//! Editing a service's environment variables: a row per variable (name, value, whether it's a
//! secret), Add and Save. It loads them once, so a refresh never overwrites what you type, and a
//! save replaces the whole set. The web panel's EnvEditor.

use std::sync::Arc;

use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{Sizable, WindowExt, h_flex, v_flex};
use gpui_kit::*;

use crate::api::EnvVar;
use crate::hub::{Error, Hub};
use crate::link::Link;
use crate::ui::muted;

type Load = Arc<dyn Fn(&Hub) -> Result<Vec<EnvVar>, Error> + Send + Sync>;
type Save = Arc<dyn Fn(&Hub, &[EnvVar]) -> Result<(), Error> + Send + Sync>;

struct Row {
    key: Entity<InputState>,
    value: Entity<InputState>,
    secret: bool,
}

pub struct EnvEditor {
    link: Link,
    save: Save,
    saved: &'static str,
    rows: Option<Vec<Row>>, // None until loaded
    error: Option<String>,
    busy: bool,
}

impl EnvEditor {
    /// load and save reach the hub; saved is what to say after a save.
    pub fn new(
        link: Link,
        load: impl Fn(&Hub) -> Result<Vec<EnvVar>, Error> + Send + Sync + 'static,
        save: impl Fn(&Hub, &[EnvVar]) -> Result<(), Error> + Send + Sync + 'static,
        saved: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (load, l): (Load, Link) = (Arc::new(load), link.clone());
        let fetch = cx.background_spawn(async move { l.call(|h| load(h)) });
        cx.spawn_in(window, async move |this, cx| {
            let result = fetch.await;
            this.update_in(cx, |e, window, cx| e.loaded(result, window, cx))
                .ok();
        })
        .detach();
        EnvEditor {
            link,
            save: Arc::new(save),
            saved,
            rows: None,
            error: None,
            busy: false,
        }
    }

    fn loaded(&mut self, result: Result<Vec<EnvVar>, Error>, window: &mut Window, cx: &mut Context<Self>) {
        match result {
            Ok(vars) => self.rows = Some(vars.iter().map(|v| row(v, window, cx)).collect()),
            Err(e) => self.error = Some(e.to_string()),
        }
        cx.notify();
    }

    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let r = row(&EnvVar::default(), window, cx);
        self.rows.get_or_insert_default().push(r);
        cx.notify();
    }

    fn vars(&self, cx: &App) -> Vec<EnvVar> {
        let text = |i: &Entity<InputState>| i.read(cx).value().to_string();
        self.rows
            .iter()
            .flatten()
            .map(|r| EnvVar {
                key: text(&r.key).trim().to_uppercase(),
                value: text(&r.value),
                secret: r.secret,
            })
            .filter(|v| !v.key.is_empty())
            .collect()
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.busy = true;
        cx.notify();
        let (vars, link, save) = (self.vars(cx), self.link.clone(), self.save.clone());
        let task = cx.background_spawn(async move { link.call(|h| save(h, &vars)) });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            this.update_in(cx, |e, window, cx| {
                e.busy = false;
                e.error = result.as_ref().err().map(|e| e.to_string());
                if result.is_ok() {
                    window.push_notification(Notification::success(e.saved), cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn row_view(&self, i: usize, r: &Row, cx: &mut Context<Self>) -> impl IntoElement {
        let secret = r.secret;
        h_flex()
            .gap_2()
            .items_center()
            .child(div().w(px(200.)).child(Input::new(&r.key)))
            .child(div().flex_1().min_w_0().child(Input::new(&r.value)))
            .child(
                Checkbox::new(SharedString::from(format!("secret-{i}")))
                    .label("secret")
                    .checked(secret)
                    .on_click(cx.listener(move |this, on: &bool, window, cx| this.mask(i, *on, window, cx))),
            )
            .child(
                Button::new(SharedString::from(format!("rm-var-{i}")))
                    .small()
                    .ghost()
                    .label("Remove")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(rows) = &mut this.rows {
                            rows.remove(i);
                        }
                        cx.notify();
                    })),
            )
    }

    fn mask(&mut self, i: usize, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(r) = self.rows.as_mut().and_then(|rows| rows.get_mut(i)) {
            r.secret = on;
            r.value.update(cx, |s, cx| s.set_masked(on, window, cx));
        }
        cx.notify();
    }
}

fn row(v: &EnvVar, window: &mut Window, cx: &mut App) -> Row {
    let key = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("KEY")
            .default_value(v.key.clone())
    });
    let value = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("value")
            .masked(v.secret)
            .default_value(v.value.clone())
    });
    Row {
        key,
        value,
        secret: v.secret,
    }
}

impl Render for EnvEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let error = self
            .error
            .clone()
            .map(|e| Alert::error("env-error", e).title("Couldn't load or save"));
        let rows: Vec<AnyElement> = match &self.rows {
            None => vec![muted("Loading…", cx).into_any_element()],
            Some(rows) if rows.is_empty() => vec![muted("No variables yet.", cx).into_any_element()],
            Some(rows) => rows
                .iter()
                .enumerate()
                .map(|(i, r)| self.row_view(i, r, cx).into_any_element())
                .collect(),
        };
        v_flex()
            .gap_2()
            .children(error)
            .children(rows)
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        Button::new("add-var")
                            .small()
                            .outline()
                            .label("Add variable")
                            .on_click(cx.listener(|this, _, window, cx| this.add(window, cx))),
                    )
                    .child(
                        Button::new("save-vars")
                            .primary()
                            .label("Save")
                            .loading(self.busy)
                            .on_click(cx.listener(|this, _, window, cx| this.submit(window, cx))),
                    ),
            )
            .child(muted(
                "Names: A–Z, 0–9 and _, starting with a letter or _. The last save replaces the whole set.",
                cx,
            ))
    }
}
