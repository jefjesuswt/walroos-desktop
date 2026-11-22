//! The integrations whose API tokens the hub keeps, sealed: Cloudflare for domains, GitHub for
//! private repos and new projects' repos. The hub checks a token with the service before it
//! keeps it, and never shows it again. Owners only.

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::form::{field, v_form};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{Sizable, WindowExt, h_flex};
use gpui_kit::*;

use super::{muted, section};
use crate::actions::{confirm, run};
use crate::api::{Cloudflare, GitHub};
use crate::hub::{Error, Hub};
use crate::page::Ctx;

const CF_SUB: &str = "Apps get public domains through a Cloudflare Tunnel per prod node: TLS at Cloudflare's edge, no open ports, works behind CGNAT.";
const CF_HINT: &str = "Create one at dash.cloudflare.com → My Profile → API Tokens with Zone · DNS · Edit, Zone · Zone · Read and Account · Cloudflare Tunnel · Edit.";
const GH_SUB: &str =
    "Nodes clone private GitHub repos with it, and new projects get their repo created on your account.";
const GH_HINT: &str = "A classic token with the repo scope, or a fine-grained one with Administration and Contents read and write.";

pub fn cloudflare(cf: &Cloudflare, input: &Entity<InputState>, ctx: &Ctx, cx: &App) -> AnyElement {
    let label = if cf.configured {
        "Replace the token"
    } else {
        "API token"
    };
    let zones = cf.configured.then(|| {
        let list = if cf.zones.is_empty() {
            "—".into()
        } else {
            cf.zones.join(", ")
        };
        muted(format!("Zones: {list}"), cx)
    });
    section("Cloudflare", CF_SUB, Some(connected(cf.configured)), cx)
        .children(zones)
        .child(form("cf", label, CF_HINT, input, ctx, save_cf))
        .into_any_element()
}

pub fn github(gh: &GitHub, input: &Entity<InputState>, ctx: &Ctx, cx: &App) -> AnyElement {
    let label = if gh.configured {
        "Replace the token"
    } else {
        "Personal access token"
    };
    let c = ctx.clone();
    let who = gh.configured.then(|| {
        h_flex()
            .justify_between()
            .items_center()
            .child(muted(format!("Repos are created as {}.", gh.login), cx))
            .child(
                Button::new("gh-remove")
                    .small()
                    .ghost()
                    .label("Remove")
                    .on_click(move |_, w, cx| confirm_remove_github(&c, w, cx)),
            )
    });
    section("GitHub", GH_SUB, Some(connected(gh.configured)), cx)
        .children(who)
        .child(form("gh", label, GH_HINT, input, ctx, save_gh))
        .into_any_element()
}

fn save_cf(hub: &Hub, token: &str) -> Result<String, Error> {
    hub.set_cloudflare(token).map(|_| "Cloudflare token saved".into())
}

fn save_gh(hub: &Hub, token: &str) -> Result<String, Error> {
    hub.set_github(token)
        .map(|g| format!("GitHub token saved for {}", g.login))
}

fn connected(ok: bool) -> AnyElement {
    let tag = if ok {
        Tag::success().child("Connected")
    } else {
        Tag::secondary().child("Not set up")
    };
    tag.small().outline().into_any_element()
}

/// The token field and its Save button. Saving clears the field.
fn form(
    id: &'static str,
    label: &'static str,
    hint: &'static str,
    input: &Entity<InputState>,
    ctx: &Ctx,
    save: fn(&Hub, &str) -> Result<String, Error>,
) -> impl IntoElement {
    let (input2, ctx) = (input.clone(), ctx.clone());
    let button = Button::new(SharedString::from(format!("{id}-save")))
        .primary()
        .label("Save")
        .on_click(move |_, window, cx| {
            let token = input2.read(cx).value().trim().to_string();
            if token.is_empty() {
                return;
            }
            let field = input2.clone();
            let clear = move |said: String, w: &mut Window, cx: &mut App| {
                field.update(cx, |s, cx| s.set_value("", w, cx));
                w.push_notification(said, cx);
            };
            run(
                &ctx,
                "Token checked".into(),
                move |hub| save(hub, &token),
                clear,
                window,
                cx,
            );
        });
    let row = h_flex()
        .gap_2()
        .child(div().flex_1().child(Input::new(input)))
        .child(button);
    v_form().child(field().label(label).description(hint).child(row))
}

fn confirm_remove_github(ctx: &Ctx, window: &mut Window, cx: &mut App) {
    let ctx = ctx.clone();
    let text = "Private GitHub repos stop cloning (unless the node has an SSH key for them) and new projects start as local git repos until you add a token again. Repos already created stay on GitHub.";
    let then = move |w: &mut Window, cx: &mut App| {
        run(
            &ctx,
            "GitHub token removed".into(),
            |hub| hub.remove_github(),
            |_, _, _| {},
            w,
            cx,
        )
    };
    confirm(
        "Remove the GitHub token?".into(),
        text.into(),
        "Remove token",
        then,
        window,
        cx,
    );
}
