//! The form's groups past the source: a static site's build, and how a release runs and proves
//! it's up.

use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::form::{field, v_form};
use gpui_kit::component::input::Input;
use gpui_kit::*;

use super::{CHECKS, RUNTIMES, SourceForm};
use crate::ui::seg;

fn check_hint(health: &str) -> &'static str {
    match health {
        "tcp" => "The port must accept connections within 90 s. For databases, caches and anything not HTTP.",
        "running" => "It must keep running for 5 s without restarting. For workers with no port.",
        _ => "A GET on the path must answer 2xx or 3xx within 90 s. For web apps and APIs.",
    }
}

impl SourceForm {
    pub(super) fn static_fields(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let me = cx.weak_entity();
        let labels: Vec<&'static str> = RUNTIMES.iter().map(|(_, l)| *l).collect();
        let runtime = seg("runtime", &labels, self.runtime, move |i, _, cx| {
            me.update(cx, |f, cx| f.pick(|f| &mut f.runtime, i, cx)).ok();
        });
        v_form()
            .columns(2)
            .child(
                field()
                    .label("Runtime")
                    .col_span(2)
                    .description("Auto uses Bun with a bun lockfile or a BUN_VERSION variable, else Node.")
                    .child(runtime),
            )
            .child(
                field()
                    .label("Install command")
                    .child(Input::new(&self.f.install)),
            )
            .child(
                field()
                    .label("Build command")
                    .description("Default: the build script in package.json.")
                    .child(Input::new(&self.f.build)),
            )
            .child(
                field()
                    .label("Publish directory")
                    .col_span(2)
                    .description("Default: the first of dist, build, out or public with an index.html.")
                    .child(Input::new(&self.f.publish)),
            )
            .child(
                field().col_span(2).child(
                    Checkbox::new("spa")
                        .label("Single-page app: paths with no file get index.html")
                        .checked(self.spa)
                        .on_click(cx.listener(|this, on: &bool, _, cx| {
                            this.spa = *on;
                            cx.notify();
                        })),
                ),
            )
    }

    pub(super) fn run_fields(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let health = CHECKS[self.health].0;
        let has_path = self.is_static() || health == "http";
        let mut form = v_form().columns(2);
        if !self.is_static() {
            let me = cx.weak_entity();
            let labels: Vec<&'static str> = CHECKS.iter().map(|(_, l)| *l).collect();
            let check = seg("health", &labels, self.health, move |i, _, cx| {
                me.update(cx, |f, cx| f.pick(|f| &mut f.health, i, cx)).ok();
            });
            form = form
                .child(
                    field()
                        .label("Health check")
                        .col_span(2)
                        .description(check_hint(health))
                        .child(check),
                )
                .child(
                    field()
                        .label("Port")
                        .col_span(if has_path { 1 } else { 2 })
                        .description("What the container listens on.")
                        .child(Input::new(&self.f.port)),
                );
        }
        if has_path {
            let span = if self.is_static() { 2 } else { 1 };
            form = form.child(
                field()
                    .label("Health check path")
                    .col_span(span)
                    .child(Input::new(&self.f.path)),
            );
        }
        if !self.is_static() {
            form = form.child(field().label("Command").col_span(2).description("Replaces the image's command. Quote arguments with spaces: sh -c \"npm run worker\".").child(Input::new(&self.f.command)));
        }
        if !self.is_image() {
            form = form.child(
                field()
                    .label("Build folder")
                    .col_span(2)
                    .description("A folder of the repo to build from, like a monorepo's service.")
                    .child(Input::new(&self.f.context)),
            );
        }
        form
    }
}
