//! The Environment tab: the app's variables, and what they can reference in its project.

use gpui_kit::*;

use super::{AppPage, refs};
use crate::api::{App as AppService, EnvVar};
use crate::env_editor::EnvEditor;
use crate::hub::Hub;
use crate::page::Ctx;
use crate::ui::{note, section};

impl AppPage {
    pub(super) fn environment(
        &mut self,
        a: &AppService,
        ctx: &Ctx,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if !ctx.manages_apps {
            return note(
                "Your role can't see this app's variables: they may hold secrets.",
                cx,
            );
        }
        if self.env.is_none() {
            let (n1, n2) = (a.name.clone(), a.name.clone());
            let load = move |h: &Hub| h.app_env(&n1);
            let save = move |h: &Hub, v: &[EnvVar]| h.set_app_env(&n2, v);
            let link = ctx.link.clone();
            self.env = Some(cx.new(|cx| {
                EnvEditor::new(
                    link,
                    load,
                    save,
                    "Variables saved: deploy to apply them",
                    window,
                    cx,
                )
            }));
            self.ask_refs(&a.project, cx);
        }
        let sub = "Passed to the container as KEY=VALUE from a 0600 file on the node, never in the command line. Encrypted in the hub; they apply on the next deploy.";
        section("Environment", sub, None, cx)
            .children(self.env.clone())
            .children(self.refs.as_ref().map(|r| refs::hints(&a.project, r, cx)))
            .into_any_element()
    }

    fn ask_refs(&mut self, project: &str, cx: &mut Context<Self>) {
        let (link, project) = (self.link.clone(), project.to_string());
        let task = cx.background_spawn(async move { link.call(|h| h.project_refs(&project)) });
        cx.spawn(async move |this, cx| {
            let refs = task.await.unwrap_or_default();
            this.update(cx, |p, cx| {
                p.refs = Some(refs);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
