//! The dialogs of an app's page: deploy a commit, edit its domains or volumes, delete it.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::component::{WindowExt, v_flex};
use gpui_kit::*;

use crate::actions::{follow, run};
use crate::api::{App as AppService, Volume};
use crate::hub::{Error, Hub};
use crate::nav;
use crate::page::Ctx;

/// Deploy with an optional commit: empty deploys the branch head. Then its log opens.
pub fn deploy(a: &AppService, ctx: &Ctx, window: &mut Window, cx: &mut App) {
    let (name, ctx) = (a.name.clone(), ctx.clone());
    let sha = cx.new(|cx| InputState::new(window, cx).placeholder("branch head"));
    let (what, image) = (
        if a.image.is_empty() {
            "Builds".into()
        } else {
            format!("Pulls {}", a.image)
        },
        !a.image.is_empty(),
    );
    let text = format!(
        "{what} on {} and starts next to the running release; the old one stops only once the new one answers.",
        a.node_name
    );
    let branch = if a.branch.is_empty() {
        "the default branch".into()
    } else {
        a.branch.clone()
    };
    window.open_alert_dialog(cx, move |dialog, _, _| {
        let (name, ctx, sha) = (name.clone(), ctx.clone(), sha.clone());
        let field = (!image).then(|| {
            v_flex().gap_1().child(Input::new(&sha)).child(format!(
                "Empty deploys the head of {branch}. A SHA (7 to 40 hex) pins a commit."
            ))
        });
        dialog
            .title(format!("Deploy {name}"))
            .description(text.clone())
            .children(field)
            .ok_text("Deploy")
            .show_cancel(true)
            .on_ok(move |_, window, cx| {
                let (n, s) = (name.clone(), sha.read(cx).value().trim().to_lowercase());
                run(
                    &ctx,
                    format!("{n} queued"),
                    move |h| h.deploy(&n, &s),
                    follow(&ctx, &name),
                    window,
                    cx,
                );
                true
            })
    });
}

/// A dialog with lines to edit: one per line, saved as a whole.
/// What a lines dialog shows and says after saving.
struct Lines {
    title: String,
    text: String,
    lines: Vec<String>,
    placeholder: String,
    done: &'static str,
}

fn edit_lines(
    d: Lines,
    save: impl Fn(&Hub, Vec<String>) -> Result<(), Error> + Clone + Send + 'static,
    ctx: &Ctx,
    window: &mut Window,
    cx: &mut App,
) {
    let Lines {
        title,
        text,
        lines,
        placeholder,
        done,
    } = d;
    let input = cx.new(|cx| {
        TextareaState::new(window, cx)
            .rows(4)
            .placeholder(placeholder)
            .default_value(lines.join("\n"))
    });
    let ctx = ctx.clone();
    window.open_alert_dialog(cx, move |dialog, _, _| {
        let (input, ctx, save) = (input.clone(), ctx.clone(), save.clone());
        dialog
            .title(title.clone())
            .description(text.clone())
            .child(Textarea::new(&input))
            .ok_text("Save")
            .show_cancel(true)
            .on_ok(move |_, window, cx| {
                let lines: Vec<String> = input
                    .read(cx)
                    .value()
                    .split(['\n', ',', ' '])
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect();
                let save = save.clone();
                run(
                    &ctx,
                    done.into(),
                    move |h| save(h, lines),
                    |_, _, _| {},
                    window,
                    cx,
                );
                true
            })
    });
}

pub(super) fn edit_domains(a: &AppService, ctx: &Ctx, window: &mut Window, cx: &mut App) {
    let name = a.name.clone();
    let text = format!(
        "Hostnames, one per line, in a zone the Cloudflare token can edit. Each gets a proxied CNAME to the tunnel and a rule to http://{}:{}. Empty removes them all.",
        a.name, a.port
    );
    let save = move |h: &Hub, lines: Vec<String>| h.set_domains(&name, &lines);
    let placeholder = format!("{}.example.com", a.name);
    let d = Lines {
        title: format!("Domains · {}", a.name),
        text,
        lines: a.domains.clone(),
        placeholder,
        done: "Domains saved",
    };
    edit_lines(d, save, ctx, window, cx);
}

pub(super) fn edit_volumes(a: &AppService, ctx: &Ctx, window: &mut Window, cx: &mut App) {
    let name = a.name.clone();
    let text = format!(
        "One per line. name:/path is a podman volume; ./dir:/path is a directory in ~/walroos-data on {}. Add :ro for read-only. They apply on the next deploy.",
        a.node_name
    );
    let save = move |h: &Hub, lines: Vec<String>| {
        let volumes: Vec<Volume> = lines.iter().map(|l| Volume::parse(l)).collect();
        h.set_volumes(&name, &volumes)
    };
    let lines = a.volumes.iter().map(Volume::line).collect();
    let placeholder = "data:/var/lib/data\n./photos:/photos:ro".to_string();
    let d = Lines {
        title: format!("Volumes · {}", a.name),
        text,
        lines,
        placeholder,
        done: "Volumes saved: deploy to apply them",
    };
    edit_lines(d, save, ctx, window, cx);
}

/// Asks first; with named volumes, whether they go too. Then back to the list.
pub(super) fn delete(a: &AppService, ctx: &Ctx, window: &mut Window, cx: &mut App) {
    let (name, ctx, named) = (
        a.name.clone(),
        ctx.clone(),
        a.volumes.iter().any(|v| !v.name.is_empty()),
    );
    let purge = Rc::new(Cell::new(false));
    let text = format!(
        "Stops the app on {} and removes its images, environment and deployment history. It can't be undone.",
        a.node_name
    );
    window.open_alert_dialog(cx, move |dialog, _, _| {
        let (name, ctx, purge2, p) = (name.clone(), ctx.clone(), purge.clone(), purge.clone());
        let check = named.then(|| {
            Checkbox::new("purge")
                .label(format!(
                    "Also delete its named volumes. Otherwise an app created again as {name} gets them back."
                ))
                .checked(purge.get())
                .on_click(move |on: &bool, window, _| {
                    p.set(*on);
                    window.refresh();
                })
        });
        dialog
            .title(format!("Delete {name}?"))
            .description(text.clone())
            .children(check)
            .ok_text("Delete app")
            .ok_variant(ButtonVariant::Danger)
            .show_cancel(true)
            .on_ok(move |_, window, cx| {
                let (n, purge) = (name.clone(), purge2.get());
                let gone = |_, _: &mut Window, cx: &mut App| nav::back(cx);
                run(
                    &ctx,
                    format!("{n} deleted"),
                    move |h| h.delete_app(&n, purge),
                    gone,
                    window,
                    cx,
                );
                true
            })
    });
}
