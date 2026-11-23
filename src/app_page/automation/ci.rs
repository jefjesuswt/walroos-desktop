//! The After CI side of the Automation tab: the deploy token the forge's workflow sends, and
//! the workflow itself.

use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::form::{field, v_form};
use gpui_kit::component::h_flex;
use gpui_kit::*;

use super::Automation;
use crate::actions::{confirm, run};
use crate::ui::{code, copy_field, muted, section};
use crate::workflow;

impl Automation {
    pub(super) fn deploy_token(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let revoke = self.app.deploy_token.then(|| {
            Button::new("revoke")
                .ghost()
                .label("Revoke")
                .on_click(cx.listener(|this, _, w, cx| this.confirm_revoke(w, cx)))
                .into_any_element()
        });
        let body = match &self.token {
            Some(t) => v_form()
                .child(
                    field().child(
                        Alert::warning(
                            "copy-token",
                            "It's shown this once. Save it as WALROOS_DEPLOY_TOKEN in the repo's secrets.",
                        )
                        .title("Copy it now"),
                    ),
                )
                .child(field().child(copy_field("token", t, "deploy token", false, cx)))
                .into_any_element(),
            None => {
                let label = if self.app.deploy_token {
                    "Replace the token"
                } else {
                    "Create a token"
                };
                let note = self
                    .app
                    .deploy_token
                    .then(|| muted("A token is set. Replacing it stops the old one.", cx));
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(
                        Button::new("make-token")
                            .primary()
                            .label(label)
                            .on_click(cx.listener(|this, _, w, cx| this.make_token(w, cx))),
                    )
                    .children(note)
                    .into_any_element()
            }
        };
        let sub =
            "Your workflow sends it to deploy this app, and only this app. Store it as a repository secret.";
        section("Deploy token", sub, revoke, cx).child(body).child(
            v_form().child(
                field()
                    .label("WALROOS_URL")
                    .description("Save it as a repository secret too (or a variable).")
                    .child(copy_field("url", &self.url(), "hooks URL", false, cx)),
            ),
        )
    }

    fn make_token(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (name, me) = (self.app.name.clone(), cx.weak_entity());
        let keep = move |token: String, _: &mut Window, cx: &mut App| {
            me.update(cx, |a, cx| {
                a.token = Some(token);
                cx.notify();
            })
            .ok();
        };
        run(
            &self.ctx,
            "Deploy token made".into(),
            move |h| h.new_deploy_token(&name),
            keep,
            window,
            cx,
        );
    }

    fn confirm_revoke(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (ctx, name, me) = (self.ctx.clone(), self.app.name.clone(), cx.weak_entity());
        let then = move |w: &mut Window, cx: &mut App| {
            let (n, me) = (name.clone(), me.clone());
            let forget = move |_, _: &mut Window, cx: &mut App| {
                me.update(cx, |a, _| a.token = None).ok();
            };
            run(
                &ctx,
                "Deploy token revoked".into(),
                move |h| h.revoke_deploy_token(&n),
                forget,
                w,
                cx,
            );
        };
        let text = format!(
            "Workflows using it can't deploy {} until you make a new one.",
            self.app.name
        );
        confirm(
            "Revoke the deploy token?".into(),
            text,
            "Revoke",
            then,
            window,
            cx,
        );
    }

    pub(super) fn workflow(&self, cx: &App) -> impl IntoElement {
        let sub = if self.app.image.is_empty() {
            "Tests, then deploys the commit that passed. Save as .github/workflows/deploy.yml."
        } else {
            "Tests, then builds and pushes the image, then deploys it. Save as .github/workflows/deploy.yml."
        };
        section("GitHub Actions workflow", sub, None, cx)
            .child(code("workflow", &workflow::github(&self.app), "workflow", cx))
            .child(muted("Replace the test step with yours. Gitea and Forgejo Actions run the same file; on GitLab, call the same URL from the last stage.", cx))
    }
}
