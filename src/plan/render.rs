use crate::plan::model::PlanGraph;
use ratatui::layout::Rect;
use ratatui::text::Line;

pub fn render_outline(graph: &PlanGraph) -> Vec<String> {
    let mut out = Vec::new();
    let roots: Vec<&crate::plan::model::PlanNode> =
        graph.nodes.iter().filter(|n| n.parent.is_none()).collect();
    for root in roots {
        render_outline_node(graph, root, 0, &mut out);
    }
    out
}

fn render_outline_node(
    graph: &PlanGraph,
    node: &crate::plan::model::PlanNode,
    depth: usize,
    out: &mut Vec<String>,
) {
    let indent = "  ".repeat(depth);
    let cross_links: Vec<String> = graph
        .cross_links
        .iter()
        .filter(|l| l.from == node.id)
        .map(|l| {
            graph
                .nodes
                .iter()
                .find(|n| n.id == l.to)
                .map(|n| n.title.clone())
                .unwrap_or_else(|| format!("node {}", l.to))
        })
        .collect();
    let link_suffix = if cross_links.is_empty() {
        String::new()
    } else {
        format!(" → {}", cross_links.join(", "))
    };
    out.push(format!("{}{}{}", indent, node.title, link_suffix));
    let children: Vec<&crate::plan::model::PlanNode> = graph
        .nodes
        .iter()
        .filter(|n| n.parent == Some(node.id))
        .collect();
    for child in children {
        render_outline_node(graph, child, depth + 1, out);
    }
}

pub fn render_flow_chart<'a>(graph: &'a PlanGraph, _area: Rect) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let roots: Vec<&crate::plan::model::PlanNode> =
        graph.nodes.iter().filter(|n| n.parent.is_none()).collect();
    for (i, root) in roots.iter().enumerate() {
        let is_last = i == roots.len() - 1;
        render_flow_node(graph, root, "", is_last, true, &mut lines);
    }
    lines
}

fn render_flow_node<'a>(
    graph: &'a PlanGraph,
    node: &'a crate::plan::model::PlanNode,
    prefix: &str,
    is_last: bool,
    is_root: bool,
    lines: &mut Vec<Line<'a>>,
) {
    let connector = if is_root {
        ""
    } else if is_last {
        "└─ "
    } else {
        "├─ "
    };
    let cross_targets: Vec<String> = graph
        .cross_links
        .iter()
        .filter(|l| l.from == node.id)
        .map(|l| {
            graph
                .nodes
                .iter()
                .find(|n| n.id == l.to)
                .map(|n| n.title.clone())
                .unwrap_or_else(|| format!("node {}", l.to))
        })
        .collect();
    let cross_marker = if cross_targets.is_empty() {
        String::new()
    } else {
        format!(" ⇢ {}", cross_targets.join(", "))
    };
    lines.push(Line::from(format!(
        "{}{}{}{}",
        prefix, connector, node.title, cross_marker
    )));
    let children: Vec<&crate::plan::model::PlanNode> = graph
        .nodes
        .iter()
        .filter(|n| n.parent == Some(node.id))
        .collect();
    let child_prefix = if is_root {
        String::new()
    } else if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}│   ", prefix)
    };
    for (i, child) in children.iter().enumerate() {
        let child_is_last = i == children.len() - 1;
        render_flow_node(graph, child, &child_prefix, child_is_last, false, lines);
    }
}

#[allow(dead_code)] // Used by main.rs (Task 4)
pub fn render_plan(f: &mut ratatui::Frame, graph: &PlanGraph, area: Rect) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::widgets::{Block, Borders, Paragraph};

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let flow_lines = render_flow_chart(graph, chunks[0]);
    let flow_widget = Paragraph::new(flow_lines)
        .block(Block::default().borders(Borders::ALL).title("Flow Chart"));
    f.render_widget(flow_widget, chunks[0]);

    let outline = render_outline(graph);
    let outline_text: Vec<Line> = outline.into_iter().map(Line::from).collect();
    let outline_widget = Paragraph::new(outline_text)
        .block(Block::default().borders(Borders::ALL).title("Outline"));
    f.render_widget(outline_widget, chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_outline_shows_hierarchy() {
        let mut g = PlanGraph::new();
        let root = g.create_node("Root", None).unwrap();
        g.create_node("Child A", Some(root)).unwrap();
        g.create_node("Child B", Some(root)).unwrap();
        let outline = render_outline(&g);
        assert!(outline.iter().any(|l| l.contains("Root")));
        assert!(outline.iter().any(|l| l.contains("Child A")));
        assert!(outline.iter().any(|l| l.contains("Child B")));
        let root_line = outline.iter().find(|l| l.contains("Root")).unwrap();
        let child_line = outline.iter().find(|l| l.contains("Child A")).unwrap();
        let root_indent = root_line.len() - root_line.trim_start().len();
        let child_indent = child_line.len() - child_line.trim_start().len();
        assert!(child_indent > root_indent);
    }

    #[test]
    fn render_outline_shows_cross_links() {
        let mut g = PlanGraph::new();
        let a = g.create_node("Alpha", None).unwrap();
        let b = g.create_node("Beta", None).unwrap();
        g.connect_create(a, b).unwrap();
        let outline = render_outline(&g);
        let alpha_line = outline.iter().find(|l| l.contains("Alpha")).unwrap();
        assert!(alpha_line.contains("→ Beta") || alpha_line.contains("-> Beta"));
    }

    #[test]
    fn render_flow_chart_returns_lines() {
        let mut g = PlanGraph::new();
        g.create_node("Root", None).unwrap();
        let lines = render_flow_chart(&g, Rect::new(0, 0, 80, 24));
        assert!(!lines.is_empty());
    }

    #[test]
    fn render_flow_chart_shows_tree_connectors() {
        let mut g = PlanGraph::new();
        let root = g.create_node("Root", None).unwrap();
        g.create_node("Child A", Some(root)).unwrap();
        g.create_node("Child B", Some(root)).unwrap();
        let lines = render_flow_chart(&g, Rect::new(0, 0, 80, 24));
        let text: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        // Root renders bare; children carry branch connectors.
        assert!(text.iter().any(|l| l == "Root"), "lines: {text:?}");
        assert!(text.iter().any(|l| l == "├─ Child A"), "lines: {text:?}");
        assert!(text.iter().any(|l| l == "└─ Child B"), "lines: {text:?}");
    }

    #[test]
    fn render_flow_chart_marks_cross_link_target() {
        let mut g = PlanGraph::new();
        let a = g.create_node("Alpha", None).unwrap();
        let b = g.create_node("Beta", None).unwrap();
        g.connect_create(a, b).unwrap();
        let lines = render_flow_chart(&g, Rect::new(0, 0, 80, 24));
        let text: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        assert!(
            text.iter().any(|l| l == "Alpha ⇢ Beta"),
            "lines: {text:?}"
        );
    }

    #[test]
    fn render_flow_chart_nests_grandchildren() {
        let mut g = PlanGraph::new();
        let root = g.create_node("Root", None).unwrap();
        let child = g.create_node("Child", Some(root)).unwrap();
        g.create_node("Grandchild", Some(child)).unwrap();
        let lines = render_flow_chart(&g, Rect::new(0, 0, 80, 24));
        let text: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        assert_eq!(
            text,
            vec![
                "Root".to_string(),
                "└─ Child".to_string(),
                "    └─ Grandchild".to_string()
            ],
            "lines: {text:?}"
        );
    }

    #[test]
    fn render_outline_empty_graph() {
        let g = PlanGraph::new();
        let outline = render_outline(&g);
        assert!(outline.is_empty());
    }
}
