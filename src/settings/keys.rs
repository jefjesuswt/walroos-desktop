//! Your SSH keys: the public keys that can open a shell in your workspaces.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::label::Label;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, WindowExt, h_flex, v_flex};
use gpui_kit::*;

use super::{muted, section};
use crate::actions::run;
use crate::api::SshKey;
use crate::page::Ctx;
use crate::when;

pub fn card(keys: &[SshKey], ctx: &Ctx, cx: &App) -> AnyElement {
    let c = ctx.clone();
    let add = Button::new("add-key")
        .small()
        .primary()
        .icon(Icon::new(IconName::Plus))
        .label("Add key")
        .on_click(move |_, window, cx| add(&c, window, cx));
    let sub =
        "Public keys that can open a shell in your workspaces. The private key never leaves your machine.";
    let list = if keys.is_empty() {
        muted("No keys yet. Without one you can't SSH into a workspace.", cx).into_any_element()
    } else {
        v_flex()
            .gap_2()
            .children(keys.iter().map(|k| row(k, ctx, cx)))
            .into_any_element()
    };
    section("SSH keys", sub, Some(add.into_any_element()), cx)
        .child(list)
        .into_any_element()
}

fn row(k: &SshKey, ctx: &Ctx, cx: &App) -> impl IntoElement {
    let (id, name, ctx) = (k.id.clone(), k.name.clone(), ctx.clone());
    let mono = cx.theme().mono_font_family.clone();
    h_flex()
        .gap_3()
        .items_center()
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .child(Label::new(k.name.clone()))
                .child(
                    div()
                        .truncate()
                        .child(muted(k.fingerprint.clone(), cx).font_family(mono)),
                ),
        )
        .child(muted(when::short(&k.created_at), cx))
        .child(
            Button::new(SharedString::from(format!("rm-key-{}", k.id)))
                .small()
                .ghost()
                .label("Remove")
                .on_click(move |_, window, cx| {
                    let id = id.clone();
                    let done = format!("Key \"{name}\" removed");
                    run(
                        &ctx,
                        done,
                        move |hub| hub.remove_ssh_key(&id),
                        |_, _, _| {},
                        window,
                        cx,
                    )
                }),
        )
}

fn add(ctx: &Ctx, window: &mut Window, cx: &mut App) {
    let name = cx.new(|cx| InputState::new(window, cx).placeholder("laptop"));
    let key = cx.new(|cx| InputState::new(window, cx).placeholder("ssh-ed25519 AAAA… you@laptop"));
    let ctx = ctx.clone();
    window.open_alert_dialog(cx, move |dialog, _, _| {
        let (name, key, ctx) = (name.clone(), key.clone(), ctx.clone());
        dialog
            .title("Add SSH key")
            .description("Paste the public key (the .pub file). Find it with cat ~/.ssh/id_ed25519.pub.")
            .child(v_flex().gap_3().child(Input::new(&name)).child(Input::new(&key)))
            .ok_text("Add key")
            .show_cancel(true)
            .on_ok(move |_, window, cx| {
                let n = name.read(cx).value().trim().to_string();
                let k = key.read(cx).value().trim().to_string();
                if n.is_empty() || k.is_empty() {
                    return false;
                }
                let done = format!("Key \"{n}\" added");
                run(
                    &ctx,
                    done,
                    move |hub| hub.add_ssh_key(&n, &k),
                    |_, _, _| {},
                    window,
                    cx,
                );
                true
            })
    });
}
