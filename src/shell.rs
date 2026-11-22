//! The window: tabs for each page, and the page that's open. A page is made the first time its
//! tab opens and kept after, so switching back shows it at once with its data, never a reload.

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
    pages: [Option<AnyView>; TABS.len()],
}

impl Shell {
    pub fn new(link: Link, cx: &mut Context<Self>) -> Self {
        let mut shell = Shell {
            link,
            tab: 0,
            pages: Default::default(),
        };
        shell.page(0, cx);
        shell
    }

    fn select(&mut self, tab: usize, cx: &mut Context<Self>) {
        self.tab = tab;
        cx.notify();
    }

    /// The tab's page, made the first time it's asked for.
    fn page(&mut self, tab: usize, cx: &mut App) -> AnyView {
        let link = &self.link;
        self.pages[tab].get_or_insert_with(|| open(tab, link, cx)).clone()
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
        let page = self.page(self.tab, cx);
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
            .child(page)
    }
}
