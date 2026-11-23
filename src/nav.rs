//! Going from a list to one thing's page and back. The shell keeps the page open over its tab;
//! any view can ask for one without holding the shell.

use gpui_kit::*;

use crate::shell::Shell;

/// A page of one thing.
#[derive(Clone)]
pub enum Detail {
    App(String),
}

struct Nav(WeakEntity<Shell>);

impl Global for Nav {}

/// Called once by the shell, so open and back reach it.
pub fn init(shell: WeakEntity<Shell>, cx: &mut App) {
    cx.set_global(Nav(shell));
}

pub fn open(detail: Detail, window: &mut Window, cx: &mut App) {
    if let Some(shell) = cx.try_global::<Nav>().map(|n| n.0.clone()) {
        shell.update(cx, |s, cx| s.open(detail, window, cx)).ok();
    }
}

pub fn back(cx: &mut App) {
    if let Some(shell) = cx.try_global::<Nav>().map(|n| n.0.clone()) {
        shell.update(cx, |s, cx| s.back(cx)).ok();
    }
}
