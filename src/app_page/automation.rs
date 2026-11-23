//! The Automation tab: how the app deploys by itself. On push, the forge's webhook deploys every
//! push to the branch; after CI, the forge's workflow runs the tests and calls the hub with a
//! deploy token once they pass. The tests always run in the forge: walroos only deploys.

use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::Button;
use gpui_kit::component::form::{field, v_form};
use gpui_kit::*;

use crate::actions::{confirm, run};
use crate::api::{App as AppService, Hooks};
use crate::page::Ctx;
use crate::ui::{copy_field, muted, note, section, seg};

mod ci;

const TRIGGERS: [(&str, &str); 3] = [
    ("manual", "Manual"),
    ("on_push", "On push"),
    ("on_ci", "After CI"),
];
const FORGES: [(&str, &str); 3] = [
    ("github", "GitHub"),
    ("gitea", "Gitea · Forgejo"),
    ("gitlab", "GitLab"),
];

pub struct Automation {
    app: AppService, // the page keeps it current
    ctx: Ctx,
    hooks: Option<Hooks>,
    secret: Option<String>, // the webhook's, once asked for
    token: Option<String>,  // a deploy token just made: shown this once
    forge: usize,
}

impl Automation {
    pub fn new(app: AppService, ctx: Ctx, cx: &mut Context<Self>) -> Self {
        let link = ctx.link.clone();
        let task = cx.background_spawn(async move { link.call(|h| h.hooks()) });
        cx.spawn(async move |this, cx| {
            let hooks = task.await.unwrap_or_default();
            this.update(cx, |a, cx| {
                a.hooks = Some(hooks);
                cx.notify();
            })
            .ok();
        })
        .detach();
        let mut this = Automation {
            app,
            ctx,
            hooks: None,
            secret: None,
            token: None,
            forge: 0,
        };
        this.ask_secret(cx);
        this
    }

    /// The page got a fresher app.
    pub fn set_app(&mut self, app: AppService, cx: &mut Context<Self>) {
        self.app = app;
        self.ask_secret(cx);
        cx.notify();
    }

    fn ask_secret(&mut self, cx: &mut Context<Self>) {
        if self.app.trigger != "on_push" || self.secret.is_some() {
            return;
        }
        self.secret = Some(String::new()); // asked
        let (link, name) = (self.ctx.link.clone(), self.app.name.clone());
        let task = cx.background_spawn(async move { link.call(|h| h.webhook_secret(&name)) });
        cx.spawn(async move |this, cx| {
            let secret = task.await.unwrap_or_default();
            this.update(cx, |a, cx| {
                a.secret = Some(secret);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn url(&self) -> String {
        match &self.hooks {
            Some(h) if !h.url.is_empty() => h.url.clone(),
            _ => "<hub URL>".into(),
        }
    }

    fn deploys(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = TRIGGERS
            .iter()
            .position(|(t, _)| *t == self.app.trigger)
            .unwrap_or(0);
        let (ctx, name) = (self.ctx.clone(), self.app.name.clone());
        let labels: Vec<&'static str> = TRIGGERS.iter().map(|(_, l)| *l).collect();
        let pick = seg("trigger", &labels, current, move |i, window, cx| {
            let (trigger, n) = (TRIGGERS[i].0, name.clone());
            let done = if trigger == "manual" {
                "Deploys are manual now"
            } else {
                "Automatic deploys on"
            };
            run(
                &ctx,
                done.into(),
                move |h| h.set_trigger(&n, trigger),
                |_, _, _| {},
                window,
                cx,
            );
        });
        let branch = if self.app.branch.is_empty() {
            "the default branch"
        } else {
            &self.app.branch
        };
        let help = match self.app.trigger.as_str() {
            "on_push" => format!("Every push to {branch} deploys that commit."),
            "on_ci" => {
                "Your workflow calls walroos once its tests pass. Nothing deploys if they fail.".into()
            }
            _ => "Only the Deploy button, the API or MCP deploy it.".into(),
        };
        let unreachable = self
            .hooks
            .as_ref()
            .filter(|h| self.app.trigger != "manual" && !h.error.is_empty())
            .map(|h| {
                Alert::warning("hooks-error", h.error.clone()).title("The forge can't reach walroos yet")
            });
        let sub = "Tests run in your forge (GitHub Actions, Gitea Actions, GitLab CI); walroos deploys.";
        section("Deploys", sub, None, cx)
            .child(pick)
            .child(muted(help, cx))
            .children(unreachable)
    }

    fn webhook(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let me = cx.weak_entity();
        let labels: Vec<&'static str> = FORGES.iter().map(|(_, l)| *l).collect();
        let forge = seg("forge", &labels, self.forge, move |i, _, cx| {
            me.update(cx, |a, cx| {
                a.forge = i;
                cx.notify();
            })
            .ok();
        });
        let (id, payload) = (
            FORGES[self.forge].0,
            format!("{}/hooks/{}/{}", self.url(), FORGES[self.forge].0, self.app.name),
        );
        let secret = match self.secret.as_deref() {
            Some(s) if !s.is_empty() => {
                copy_field("secret", s, "webhook secret", true, cx).into_any_element()
            }
            _ => muted("Loading…", cx).into_any_element(),
        };
        let rotate = Button::new("rotate")
            .outline()
            .label("New secret")
            .on_click(cx.listener(|this, _, w, cx| this.confirm_rotate(w, cx)));
        let secret_label = if id == "gitlab" { "Secret token" } else { "Secret" };
        let sub =
            "Add it in the repo's settings → Webhooks. Only push events; content type application/json.";
        section("Webhook", sub, Some(rotate.into_any_element()), cx)
            .child(forge)
            .child(
                v_form()
                    .child(field().label("Payload URL").child(copy_field(
                        "payload",
                        &payload,
                        "payload URL",
                        false,
                        cx,
                    )))
                    .child(field().label(secret_label).child(secret)),
            )
    }

    fn confirm_rotate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (ctx, name, me) = (self.ctx.clone(), self.app.name.clone(), cx.weak_entity());
        let then = move |w: &mut Window, cx: &mut App| {
            let (n, me) = (name.clone(), me.clone());
            let keep = move |secret: String, _: &mut Window, cx: &mut App| {
                me.update(cx, |a, cx| {
                    a.secret = Some(secret);
                    cx.notify();
                })
                .ok();
            };
            run(
                &ctx,
                "New secret: update it in the forge".into(),
                move |h| h.rotate_webhook_secret(&n),
                keep,
                w,
                cx,
            );
        };
        let text = "Pushes signed with the old one stop deploying until you paste the new one in the forge.";
        confirm(
            "Make a new secret?".into(),
            text.into(),
            "New secret",
            then,
            window,
            cx,
        );
    }
}

impl Render for Automation {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.ctx.manages_apps {
            return note("Your role can't change how this app deploys.", cx);
        }
        let mut out = gpui_kit::component::v_flex().gap_4().child(self.deploys(cx));
        match self.app.trigger.as_str() {
            "on_push" => out = out.child(self.webhook(cx)),
            "on_ci" => out = out.child(self.deploy_token(cx)).child(self.workflow(cx)),
            _ => {}
        }
        out.into_any_element()
    }
}
