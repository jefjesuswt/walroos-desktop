//! The window: tabs for each page, and the page that's open. A page is made the first time its
//! tab opens and kept after, so switching back shows it at once with its data, never a reload.

use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::*;

use crate::link::Link;
use crate::page::Page;
use crate::settings::{Settings, Switched};
use crate::{deployments, nodes, projects};

const TABS: [&str; 4] = ["Nodes", "Projects", "Deployments", "Settings"];
const SETTINGS: usize = 3;

pub struct Shell {
    link: Link,
    tab: usize,
    pages: [Option<AnyView>; TABS.len()],
    _switched: Option<Subscription>,
}

impl Shell {
    pub fn new(link: Link) -> Self {
        Shell {
            link,
            tab: 0,
            pages: Default::default(),
            _switched: None,
        }
    }

    fn select(&mut self, tab: usize, cx: &mut Context<Self>) {
        self.tab = tab;
        cx.notify();
    }

    /// The tab's page, made the first time it's asked for.
    fn page(&mut self, tab: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyView {
        if let Some(p) = &self.pages[tab] {
            return p.clone();
        }
        let link = self.link.clone();
        let page = if tab == SETTINGS {
            let settings = cx.new(|cx| Settings::new(link, window, cx));
            self._switched = Some(cx.subscribe(&settings, |this, _, _: &Switched, cx| this.restart(cx)));
            settings.into()
        } else {
            open(tab, link, cx)
        };
        self.pages[tab] = Some(page.clone());
        page
    }

    /// Another hub: every page but Settings is made again the next time it shows.
    fn restart(&mut self, cx: &mut Context<Self>) {
        for (tab, page) in self.pages.iter_mut().enumerate() {
            if tab != SETTINGS {
                *page = None;
            }
        }
        cx.notify();
    }
}

fn open(tab: usize, link: Link, cx: &mut App) -> AnyView {
    match tab {
        0 => cx.new(|cx| Page::new(nodes::spec(), link, cx)).into(),
        1 => cx.new(|cx| Page::new(projects::spec(), link, cx)).into(),
        _ => cx.new(|cx| Page::new(deployments::spec(), link, cx)).into(),
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = self.page(self.tab, window, cx);
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
