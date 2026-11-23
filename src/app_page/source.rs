//! The General tab's form: where the app comes from (a repo with a Dockerfile, a static site or
//! a published image) and how a release proves it's up. The web panel's SourceCard, RunFields
//! and StaticFields.

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::form::{field, v_form};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{Disableable, h_flex};
use gpui_kit::*;
use serde_json::{Value, json};

use crate::actions::run;
use crate::api::{App as AppService, StaticBuild};
use crate::command;
use crate::page::Ctx;
use crate::ui::{section, seg};

mod fields;

const KINDS: [&str; 2] = ["Git repository", "Docker image"];
const BUILDS: [&str; 2] = ["Dockerfile", "Static site"];
pub(super) const RUNTIMES: [(&str, &str); 3] = [("", "Auto"), ("node", "Node"), ("bun", "Bun")];
pub(super) const CHECKS: [(&str, &str); 3] = [("http", "HTTP"), ("tcp", "TCP"), ("running", "Running")];

/// The form is done (saved, or cancelled): the page shows the summary again.
pub struct Done;

pub struct SourceForm {
    app: String,
    ctx: Ctx,
    draft: bool,
    kind: usize,  // KINDS
    build: usize, // BUILDS
    runtime: usize,
    health: usize,
    spa: bool,
    f: Fields,
}

struct Fields {
    repo: Entity<InputState>,
    image: Entity<InputState>,
    branch: Entity<InputState>,
    dockerfile: Entity<InputState>,
    install: Entity<InputState>,
    build: Entity<InputState>,
    publish: Entity<InputState>,
    port: Entity<InputState>,
    path: Entity<InputState>,
    command: Entity<InputState>,
    context: Entity<InputState>,
}

impl EventEmitter<Done> for SourceForm {}

impl SourceForm {
    pub fn new(a: &AppService, ctx: Ctx, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut input = |value: &str, hint: &str| {
            let (value, hint) = (value.to_string(), hint.to_string());
            cx.new(|cx| InputState::new(window, cx).placeholder(hint).default_value(value))
        };
        let s = a.static_build.clone().unwrap_or(StaticBuild {
            spa: true,
            ..Default::default()
        });
        let port = if a.port == 0 {
            String::new()
        } else {
            a.port.to_string()
        };
        let f = Fields {
            repo: input(&a.repo_url, "https://github.com/me/app.git"),
            image: input(&a.image, "ghcr.io/owner/app:latest"),
            branch: input(&a.branch, "default branch"),
            dockerfile: input(&a.dockerfile, "Dockerfile"),
            install: input(&s.install, "npm ci"),
            build: input(&s.build, "npm run build"),
            publish: input(&s.publish, "dist"),
            port: input(&port, "3000"),
            path: input(&a.health_path, "/"),
            command: input(&command::join(&a.command), "the image's own"),
            context: input(&a.context, "the repo's root"),
        };
        SourceForm {
            app: a.name.clone(),
            ctx,
            draft: a.is_draft(),
            kind: usize::from(!a.image.is_empty()),
            build: usize::from(a.static_build.is_some()),
            runtime: RUNTIMES.iter().position(|(r, _)| *r == s.runtime).unwrap_or(0),
            health: CHECKS.iter().position(|(h, _)| *h == a.health).unwrap_or(0),
            spa: s.spa,
            f,
        }
    }

    fn text(&self, i: &Entity<InputState>, cx: &App) -> String {
        i.read(cx).value().trim().to_string()
    }

    fn is_image(&self) -> bool {
        self.kind == 1
    }

    fn is_static(&self) -> bool {
        !self.is_image() && self.build == 1
    }

    /// What PUT /api/apps/{name}/source takes, as the web panel builds it.
    fn body(&self, cx: &App) -> Value {
        let t = |i| self.text(i, cx);
        let health = match CHECKS[self.health].0 {
            "http" => "",
            h => h,
        };
        let port: u16 = t(&self.f.port).parse().unwrap_or(0);
        let cmd = command::split(&t(&self.f.command));
        if self.is_image() {
            return json!({ "image": t(&self.f.image), "health": health, "health_path": t(&self.f.path),
                "port": port, "command": cmd, "context": "" });
        }
        let base = json!({ "repo_url": t(&self.f.repo), "branch": t(&self.f.branch),
            "health_path": t(&self.f.path), "context": t(&self.f.context) });
        let mut body = base.as_object().cloned().unwrap_or_default();
        if self.is_static() {
            let build = StaticBuild {
                runtime: RUNTIMES[self.runtime].0.into(),
                install: t(&self.f.install),
                build: t(&self.f.build),
                publish: t(&self.f.publish),
                spa: self.spa,
            };
            body.extend([
                ("static".into(), json!(build)),
                ("health".into(), json!("")),
                ("command".into(), json!([])),
                ("port".into(), json!(0)),
            ]);
        } else {
            body.extend([
                ("dockerfile".into(), json!(t(&self.f.dockerfile))),
                ("health".into(), json!(health)),
                ("port".into(), json!(port)),
                ("command".into(), json!(cmd)),
            ]);
        }
        Value::Object(body)
    }

    /// The source is set, and a release has something to prove itself with.
    fn ready(&self, cx: &App) -> bool {
        let src = if self.is_image() {
            &self.f.image
        } else {
            &self.f.repo
        };
        let port_ok = self.is_static()
            || CHECKS[self.health].0 == "running"
            || self.text(&self.f.port, cx).parse::<u16>().is_ok_and(|p| p > 0);
        !self.text(src, cx).is_empty() && port_ok
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (name, body, me) = (self.app.clone(), self.body(cx), cx.weak_entity());
        let done = if self.draft {
            "Source set: deploy it when ready"
        } else {
            "Source saved: it applies on the next deploy"
        };
        let then = move |_, _: &mut Window, cx: &mut App| {
            me.update(cx, |_, cx| cx.emit(Done)).ok();
        };
        run(
            &self.ctx,
            done.into(),
            move |hub| hub.set_source(&name, body),
            then,
            window,
            cx,
        );
    }

    fn pick(&mut self, what: fn(&mut Self) -> &mut usize, i: usize, cx: &mut Context<Self>) {
        *what(self) = i;
        cx.notify();
    }

    fn source_fields(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let me = cx.weak_entity();
        let kind = seg("kind", &KINDS, self.kind, move |i, _, cx| {
            me.update(cx, |f, cx| f.pick(|f| &mut f.kind, i, cx)).ok();
        });
        let mut form = v_form()
            .columns(2)
            .child(field().label("Source").col_span(2).child(kind));
        if self.is_image() {
            return form.child(
                field()
                    .label("Image")
                    .col_span(2)
                    .description("Fully qualified, with its registry.")
                    .child(Input::new(&self.f.image)),
            );
        }
        let me = cx.weak_entity();
        let build = seg("build", &BUILDS, self.build, move |i, _, cx| {
            me.update(cx, |f, cx| f.pick(|f| &mut f.build, i, cx)).ok();
        });
        form = form
            .child(field().label("Repository").col_span(2).description("A private GitHub repo clones with the token in Settings → GitHub; any other, with an SSH key on the node.").child(Input::new(&self.f.repo)))
            .child(field().label("Build").col_span(2).child(build))
            .child(field().label("Branch").col_span(if self.is_static() { 2 } else { 1 }).child(Input::new(&self.f.branch)));
        if !self.is_static() {
            form = form.child(field().label("Dockerfile").child(Input::new(&self.f.dockerfile)));
        }
        form
    }
}

impl Render for SourceForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let me = cx.weak_entity();
        let cancel = (!self.draft).then(|| {
            Button::new("cancel-source")
                .label("Cancel")
                .on_click(move |_, _, cx| {
                    me.update(cx, |_, cx| cx.emit(Done)).ok();
                })
        });
        let save = Button::new("save-source")
            .primary()
            .label("Save")
            .disabled(!self.ready(cx))
            .on_click(cx.listener(|this, _, window, cx| this.save(window, cx)));
        let statics = self.is_static().then(|| self.static_fields(cx));
        section(
            "Source",
            "Where the app comes from. It applies on the next deploy.",
            None,
            cx,
        )
        .child(self.source_fields(cx))
        .children(statics)
        .child(self.run_fields(cx))
        .child(h_flex().gap_2().justify_end().children(cancel).child(save))
    }
}
