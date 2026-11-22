//! The Nodes page: every node of the fleet with its health, the ones that need you first.

use gpui_kit::component::v_flex;
use gpui_kit::*;

use crate::api::{Node, sort_nodes};
use crate::node_card::node_card;
use crate::page::{Spec, empty};

pub fn spec() -> Spec<Vec<Node>> {
    Spec {
        title: "Nodes",
        fetch: |hub| {
            let mut nodes = hub.nodes()?;
            sort_nodes(&mut nodes);
            Ok(nodes)
        },
        body: |nodes, _, cx| body(nodes, cx),
    }
}

fn body(nodes: &[Node], cx: &App) -> AnyElement {
    if nodes.is_empty() {
        return empty("No nodes yet. Join one with `walroos node join`.", cx);
    }
    v_flex()
        .gap_3()
        .children(nodes.iter().map(|n| node_card(n, cx)))
        .into_any_element()
}
