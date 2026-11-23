//! The tabs of an app's page that keep no state of their own: Domains, Deployments, Advanced.

use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_kit::component::v_flex;
use gpui_kit::*;

use super::dialogs;
use crate::api::{App as AppService, Deployment};
use crate::deployments;
use crate::page::{Ctx, card};
use crate::ui::{facts, mono, muted, note, section};

pub fn domains(a: &AppService, ctx: &Ctx, cx: &App) -> AnyElement {
    let edit = ctx.manages_apps.then(|| {
        let (a, ctx) = (a.clone(), ctx.clone());
        Button::new("edit-domains")
            .outline()
            .label("Edit")
            .on_click(move |_, w, cx| dialogs::edit_domains(&a, &ctx, w, cx))
            .into_any_element()
    });
    let list = if a.domains.is_empty() {
        let port = if a.port == 0 {
            "port".into()
        } else {
            a.port.to_string()
        };
        muted(
            format!(
                "None yet: the app is only reachable inside its project, as {}:{port}.",
                a.name
            ),
            cx,
        )
        .into_any_element()
    } else {
        v_flex()
            .gap_1()
            .children(a.domains.iter().map(|d| mono(format!("https://{d}"), cx)))
            .into_any_element()
    };
    let sub = format!(
        "Public, with TLS, through {}'s Cloudflare Tunnel. No ports open on the node.",
        a.node_name
    );
    section("Domains", sub, edit, cx).child(list).into_any_element()
}

pub fn deployments(a: &AppService, deps: &[Deployment], ctx: &Ctx, cx: &App) -> AnyElement {
    if deps.is_empty() {
        let text = if a.is_draft() {
            "Set a source in General, then Deploy.".into()
        } else {
            let what = if a.image.is_empty() {
                "builds the branch head"
            } else {
                "pulls the image"
            };
            format!("Nothing deployed yet. Deploy {what} on {}.", a.node_name)
        };
        return note(text, cx);
    }
    card(cx)
        .gap_0()
        .p_0()
        .children(deps.iter().map(|d| deployments::row(d, &a.name, false, ctx, cx)))
        .into_any_element()
}

pub fn advanced(a: &AppService, ctx: &Ctx, cx: &App) -> AnyElement {
    let edit = ctx.manages_apps.then(|| {
        let (a, ctx) = (a.clone(), ctx.clone());
        Button::new("edit-volumes")
            .outline()
            .label("Edit")
            .on_click(move |_, w, cx| dialogs::edit_volumes(&a, &ctx, w, cx))
            .into_any_element()
    });
    let volumes = if a.volumes.is_empty() {
        muted("None: every deploy starts from the image's files.", cx).into_any_element()
    } else {
        v_flex()
            .gap_1()
            .children(a.volumes.iter().map(|v| mono(v.line(), cx)))
            .into_any_element()
    };
    let r = a.running.as_ref();
    let port = if a.port == 0 {
        "—".into()
    } else {
        a.port.to_string()
    };
    let runtime = facts(
        vec![
            ("Container", r.map_or("—".into(), |r| r.container.clone())),
            ("Image", r.map_or("—".into(), |r| r.image.clone())),
            ("In the project", format!("{}:{port}", a.name)),
            ("Limits", "2 GB of memory, 2048 processes".into()),
        ],
        cx,
    );
    let vol_sub = "Data that outlives releases. With any, a deploy stops the running release before starting the new one.";
    let mut out = v_flex()
        .gap_4()
        .child(section("Volumes", vol_sub, edit, cx).child(volumes))
        .child(section("Runtime", format!("What runs on {} now.", a.node_name), None, cx).child(runtime));
    if ctx.manages_apps {
        let (a2, ctx) = (a.clone(), ctx.clone());
        let delete = Button::new("delete-app")
            .with_variant(ButtonVariant::Danger)
            .label("Delete app")
            .on_click(move |_, w, cx| dialogs::delete(&a2, &ctx, w, cx));
        let sub = format!(
            "Stops it on {} and removes its images, environment and deployment history.",
            a.node_name
        );
        out = out.child(section("Delete app", sub, Some(delete.into_any_element()), cx));
    }
    out.into_any_element()
}
