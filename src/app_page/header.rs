//! The top of an app's page: its name, state and where it runs, and what you can do to it.

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{Disableable, StyledExt, h_flex, v_flex};
use gpui_kit::*;

use super::{AppPage, dialogs};
use crate::actions::app_menu;
use crate::api::App as AppService;
use crate::page::Ctx;
use crate::status::app_state;
use crate::ui::{mono, muted};

impl AppPage {
    pub(super) fn header(&self, a: &AppService, ctx: &Ctx, cx: &mut Context<Self>) -> impl IntoElement {
        let mut meta = vec![format!("on {}", a.node_name)];
        if let Some(r) = a.running.as_ref().filter(|r| !r.sha.is_empty()) {
            meta.push(format!("live {}", r.sha.chars().take(7).collect::<String>()));
        }
        if !a.description.is_empty() {
            meta.insert(0, a.description.clone());
        }
        let actions = ctx.manages_apps.then(|| {
            let (a2, c2) = (a.clone(), ctx.clone());
            let label = if a.running.is_some() { "Redeploy" } else { "Deploy" };
            h_flex().gap_2().child(app_menu(a, ctx)).child(
                Button::new("deploy")
                    .primary()
                    .label(label)
                    .disabled(a.node_down || a.is_draft())
                    .on_click(move |_, w, cx| dialogs::deploy(&a2, &c2, w, cx)),
            )
        });
        h_flex()
            .justify_between()
            .items_center()
            .gap_4()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(mono(a.name.clone(), cx).text_xl().font_semibold())
                            .child(app_state(a).tag()),
                    )
                    .child(muted(meta.join(" · "), cx)),
            )
            .children(actions)
    }
}
