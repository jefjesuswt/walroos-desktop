//! Under an app's variables: what their values can reference in its project, like
//! ${{shop-db.URL}} or ${{shared.STRIPE_KEY}}. Clicking one copies it.

use gpui_kit::component::button::Button;
use gpui_kit::component::{Sizable, h_flex, v_flex};
use gpui_kit::*;

use crate::api::Ref;
use crate::ui::{copy, mono, muted};

pub fn hints(project: &str, refs: &[Ref], cx: &App) -> impl IntoElement + use<> {
    let help = format!(
        "Reference another service of {project} or a shared variable: the value is filled in on each deploy, so it follows changes."
    );
    v_flex()
        .gap_2()
        .child(muted(help, cx))
        .children(refs.iter().map(|r| row(r, cx)))
}

fn row(r: &Ref, cx: &App) -> impl IntoElement + use<> {
    let keys: Vec<AnyElement> = if r.keys.is_empty() {
        let none = if r.service == "shared" {
            "none yet: add them with Variables on the project"
        } else {
            "nothing to reference"
        };
        vec![muted(none, cx).into_any_element()]
    } else {
        r.keys
            .iter()
            .map(|k| {
                let reference = format!("${{{{{}.{k}}}}}", r.service);
                Button::new(SharedString::from(format!("ref-{}-{k}", r.service)))
                    .xsmall()
                    .outline()
                    .label(k.clone())
                    .on_click(move |_, w, cx| copy(&reference, &reference, w, cx))
                    .into_any_element()
            })
            .collect()
    };
    h_flex()
        .gap_3()
        .items_start()
        .child(mono(r.service.clone(), cx).text_sm().w(px(128.)).flex_none())
        .child(h_flex().flex_1().flex_wrap().gap_1().children(keys))
}
