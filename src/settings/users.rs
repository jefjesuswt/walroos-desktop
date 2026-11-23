//! Who can use the hub and with which role. Owners only.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::label::Label;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::radio::{Radio, RadioGroup};
use gpui_kit::component::{ActiveTheme, Disableable, Icon, Sizable, WindowExt, h_flex, v_flex};
use gpui_kit::*;

use crate::actions::{confirm, run};
use crate::api::{Member, ROLES, User};
use crate::page::Ctx;
use crate::ui::{muted, section};
use crate::when;

pub fn card(users: &[Member], me: &User, ctx: &Ctx, cx: &App) -> AnyElement {
    let c = ctx.clone();
    let add = Button::new("add-user")
        .small()
        .primary()
        .icon(Icon::new(IconName::Plus))
        .label("Add user")
        .on_click(move |_, window, cx| add(&c, window, cx));
    let sub = "Tailscale logins that can use this hub. They must already be on your tailnet.";
    section("Users", sub, Some(add.into_any_element()), cx)
        .child(
            v_flex()
                .gap_2()
                .children(users.iter().map(|m| row(m, me, ctx, cx))),
        )
        .into_any_element()
}

fn help(role: &str) -> &'static str {
    ROLES.iter().find(|(r, _)| *r == role).map_or("", |(_, h)| h)
}

fn row(m: &Member, me: &User, ctx: &Ctx, cx: &App) -> impl IntoElement {
    let you = m.login == me.login;
    let (login, c) = (m.login.clone(), ctx.clone());
    let remove = Button::new(SharedString::from(format!("rm-{}", m.login)))
        .small()
        .ghost()
        .label("Remove")
        .on_click(move |_, window, cx| confirm_remove(&login, &c, window, cx));
    h_flex()
        .gap_3()
        .items_center()
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .child(Label::new(m.login.clone()).font_family(cx.theme().mono_font_family.clone()))
                .child(muted(if you { "you" } else { help(&m.role) }, cx)),
        )
        .child(role_menu(m, you, ctx))
        .child(muted(when::short(&m.created_at), cx))
        .children((!you).then_some(remove))
}

/// The role, as a menu to change it. Your own is changed by another owner.
fn role_menu(m: &Member, you: bool, ctx: &Ctx) -> impl IntoElement {
    let (login, current, ctx) = (m.login.clone(), m.role.clone(), ctx.clone());
    Button::new(SharedString::from(format!("role-{}", m.login)))
        .small()
        .outline()
        .label(m.role.clone())
        .dropdown_caret(!you)
        .disabled(you)
        .dropdown_menu(move |menu, _, _| {
            ROLES.iter().fold(menu, |menu, (role, _)| {
                let (login, ctx) = (login.clone(), ctx.clone());
                menu.item(
                    PopupMenuItem::new(*role)
                        .checked(*role == current)
                        .on_click(move |_, w, cx| {
                            let l = login.clone();
                            let done = format!("{login} is now {role}");
                            run(&ctx, done, move |hub| hub.set_role(&l, role), |_, _, _| {}, w, cx)
                        }),
                )
            })
        })
}

fn confirm_remove(login: &str, ctx: &Ctx, window: &mut Window, cx: &mut App) {
    let (l, ctx) = (login.to_string(), ctx.clone());
    let text = "They can't use the hub anymore and their SSH keys go with them. Someone who still owns workspaces can't be removed until those are deleted.";
    let then = move |w: &mut Window, cx: &mut App| {
        let l2 = l.clone();
        run(
            &ctx,
            format!("{l} removed"),
            move |hub| hub.remove_user(&l2),
            |_, _, _| {},
            w,
            cx,
        )
    };
    confirm(
        format!("Remove {login}?"),
        text.into(),
        "Remove user",
        then,
        window,
        cx,
    );
}

fn add(ctx: &Ctx, window: &mut Window, cx: &mut App) {
    let login = cx.new(|cx| InputState::new(window, cx).placeholder("friend@github"));
    let role = Rc::new(Cell::new(ROLES.len() - 1)); // developer
    let ctx = ctx.clone();
    window.open_alert_dialog(cx, move |dialog, _, _| {
        let pick = role.clone();
        let radios = RadioGroup::vertical("role")
            .selected_index(Some(role.get()))
            .children(ROLES.map(|(r, h)| Radio::new(r).label(format!("{r}: {h}"))))
            .on_click(move |ix, window, _| {
                pick.set(*ix);
                window.refresh();
            });
        let (login, role, ctx) = (login.clone(), role.clone(), ctx.clone());
        dialog
            .title("Add user")
            .description("Their Tailscale login, as the Tailscale admin console shows it.")
            .child(v_flex().gap_3().child(Input::new(&login)).child(radios))
            .ok_text("Add user")
            .show_cancel(true)
            .on_ok(move |_, window, cx| {
                let l = login.read(cx).value().trim().to_string();
                if !l.contains('@') {
                    return false;
                }
                let r = ROLES[role.get()].0;
                let done = format!("{l} added as {r}");
                run(
                    &ctx,
                    done,
                    move |hub| hub.add_user(&l, r),
                    |_, _, _| {},
                    window,
                    cx,
                );
                true
            })
    });
}
