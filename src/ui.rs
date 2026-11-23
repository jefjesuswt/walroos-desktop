//! Small pieces every screen builds with: a section card, muted text, a segmented choice, a
//! value you can copy, a list of facts, a block of code.

use gpui_kit::component::button::{Button, ButtonGroup, ButtonVariants};
use gpui_kit::component::label::Label;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme, Selectable, Sizable, StyledExt, WindowExt, h_flex, v_flex};
use gpui_kit::*;

use crate::page::card;

/// A card with a title, what it's for, an action on the right, then its content.
pub fn section(
    title: &'static str,
    sub: impl Into<SharedString>,
    action: Option<AnyElement>,
    cx: &App,
) -> Div {
    let head = h_flex()
        .gap_4()
        .items_start()
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(Label::new(title).font_semibold())
                .child(muted(sub, cx)),
        )
        .children(action);
    card(cx).child(head)
}

/// Secondary text: a hint, a date, what a row is.
pub fn muted(text: impl Into<SharedString>, cx: &App) -> Label {
    Label::new(text).text_sm().text_color(cx.theme().muted_foreground)
}

/// Text in the monospace font: names, paths, commands.
pub fn mono(text: impl Into<SharedString>, cx: &App) -> Label {
    Label::new(text).font_family(cx.theme().mono_font_family.clone())
}

/// One of a few options, as joined buttons; on_pick gets the chosen index.
pub fn seg(
    id: impl Into<ElementId>,
    options: &[&'static str],
    current: usize,
    on_pick: impl Fn(usize, &mut Window, &mut App) + 'static,
) -> ButtonGroup {
    let id = id.into();
    ButtonGroup::new(id.clone())
        .small()
        .children(options.iter().enumerate().map(|(i, o)| {
            let b = Button::new(SharedString::from(format!("{id}-{i}")))
                .label(*o)
                .selected(i == current);
            if i == current { b.primary() } else { b.outline() }
        }))
        .on_click(move |picked: &Vec<usize>, window, cx| {
            if let Some(i) = picked.first() {
                on_pick(*i, window, cx);
            }
        })
}

/// Copies text to the clipboard and says so.
pub fn copy(text: &str, what: &str, window: &mut Window, cx: &mut App) {
    cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
    window.push_notification(Notification::success(format!("Copied {what}")), cx);
}

/// A value in a box with a Copy button; masked shows dots instead (a secret).
pub fn copy_field(
    id: &str,
    value: &str,
    what: &'static str,
    masked: bool,
    cx: &App,
) -> impl IntoElement + use<> {
    let shown = if masked {
        "••••••••••••••••".to_string()
    } else {
        value.to_string()
    };
    let value = value.to_string();
    h_flex()
        .gap_2()
        .items_center()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .px_3()
                .py_1()
                .rounded(cx.theme().radius)
                .bg(cx.theme().muted)
                .truncate()
                .child(mono(shown, cx).text_sm()),
        )
        .child(
            Button::new(SharedString::from(format!("copy-{id}")))
                .small()
                .outline()
                .label("Copy")
                .on_click(move |_, window, cx| copy(&value, what, window, cx)),
        )
}

/// Facts as label and value rows (a description list).
pub fn facts(rows: Vec<(&'static str, String)>, cx: &App) -> impl IntoElement + use<> {
    v_flex().gap_1().children(rows.into_iter().map(|(k, v)| {
        h_flex()
            .gap_3()
            .items_start()
            .child(muted(k, cx).w(px(128.)).flex_none())
            .child(div().flex_1().min_w_0().child(mono(v, cx).text_sm()))
    }))
}

/// A block of code to read and copy.
pub fn code(id: &str, text: &str, what: &'static str, cx: &App) -> impl IntoElement + use<> {
    let value = text.to_string();
    v_flex()
        .gap_2()
        .child(
            div()
                .p_3()
                .rounded(cx.theme().radius)
                .bg(cx.theme().muted)
                .font_family(cx.theme().mono_font_family.clone())
                .text_size(cx.theme().mono_font_size)
                .child(text.to_string()),
        )
        .child(
            h_flex().justify_end().child(
                Button::new(SharedString::from(format!("copy-{id}")))
                    .small()
                    .outline()
                    .label("Copy")
                    .on_click(move |_, window, cx| copy(&value, what, window, cx)),
            ),
        )
}

/// A muted note in place of a tab your role can't use, or that has nothing yet.
pub fn note(text: impl Into<SharedString>, cx: &App) -> AnyElement {
    card(cx).child(muted(text, cx)).into_any_element()
}
