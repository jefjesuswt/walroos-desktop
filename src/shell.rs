//! The window: tabs for each page, and the page that's open. Only the open page polls the hub;
//! switching tabs drops the old one.

use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::*;

use crate::link::Link;
use crate::page::Page;
use crate::{nodes, projects};

const TABS: [&str; 2] = ["Nodes", "Projects"];

pub struct Shell {
    link: Link,
    tab: usize,
    page: AnyView,
}

impl Shell {
    pub fn new(link: Link, cx: &mut Context<Self>) -> Self {
        let page = open(0, &link, cx);
        Shell { link, tab: 0, page }
    }

    fn select(&mut self, tab: usize, cx: &mut Context<Self>) {
        if tab != self.tab {
            self.tab = tab;
            self.page = open(tab, &self.link, cx);
            cx.notify();
        }
    }
}

fn open(tab: usize, link: &Link, cx: &mut App) -> AnyView {
    let link = link.clone();
    match tab {
        0 => cx.new(|cx| Page::new(nodes::spec(), link, cx)).into(),
        _ => cx.new(|cx| Page::new(projects::spec(), link, cx)).into(),
    }
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tabs = TabBar::new("pages")
            .underline()
            .px_4()
            .selected_index(self.tab)
            .children(TABS.map(|t| Tab::new().label(t)))
            .on_click(cx.listener(|this, ix: &usize, _, cx| this.select(*ix, cx)));
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(tabs)
            .child(self.page.clone())
    }
}
