use crate::plan::model::PlanGraph;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn children(graph: &PlanGraph, id: u64) -> Vec<&crate::plan::model::PlanNode> {
    graph.nodes.iter().filter(|n| n.parent == Some(id)).collect()
}

fn roots(graph: &PlanGraph) -> Vec<&crate::plan::model::PlanNode> {
    graph.nodes.iter().filter(|n| n.parent.is_none()).collect()
}

/// Titles of cross-link targets for `id` (the escape-hatch connections).
fn cross_targets(graph: &PlanGraph, id: u64) -> Vec<String> {
    graph
        .cross_links
        .iter()
        .filter(|l| l.from == id)
        .map(|l| {
            graph
                .nodes
                .iter()
                .find(|n| n.id == l.to)
                .map(|n| n.title.clone())
                .unwrap_or_else(|| format!("node {}", l.to))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tree (Outline pane): `├─` / `└─` / `│` with `⇢ target` cross-link markers.
// Each line carries its node id for selection highlighting.
// ---------------------------------------------------------------------------

pub fn render_tree(graph: &PlanGraph) -> Vec<(u64, Line<'static>)> {
    let mut lines = Vec::new();
    let all_roots = roots(graph);
    for (i, root) in all_roots.iter().enumerate() {
        let is_last = i == all_roots.len() - 1;
        render_tree_node(graph, root, "", is_last, true, &mut lines);
    }
    lines
}

fn render_tree_node(
    graph: &PlanGraph,
    node: &crate::plan::model::PlanNode,
    prefix: &str,
    is_last: bool,
    is_root: bool,
    lines: &mut Vec<(u64, Line<'static>)>,
) {
    let connector = if is_root {
        ""
    } else if is_last {
        "└─ "
    } else {
        "├─ "
    };
    let targets = cross_targets(graph, node.id);
    let cross_marker = if targets.is_empty() {
        String::new()
    } else {
        format!(" ⇢ {}", targets.join(", "))
    };
    lines.push((
        node.id,
        Line::from(format!("{prefix}{connector}{}{cross_marker}", node.title)),
    ));
    let kids = children(graph, node.id);
    let child_prefix = if is_root {
        String::new()
    } else if is_last {
        format!("{prefix}    ")
    } else {
        format!("{prefix}│   ")
    };
    for (i, child) in kids.iter().enumerate() {
        let child_is_last = i == kids.len() - 1;
        render_tree_node(graph, child, &child_prefix, child_is_last, false, lines);
    }
}

// ---------------------------------------------------------------------------
// Boxes (Flow pane): top-down box layout with elbow connectors.
// ---------------------------------------------------------------------------

/// Vertical rows between a parent box bottom and its children tops.
pub const LINK_H: usize = 3;
/// Horizontal gap between sibling subtrees.
pub const CHILD_GAP: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxRect {
    pub id: u64,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

fn box_size(graph: &PlanGraph, id: u64) -> (usize, usize) {
    let node = graph
        .nodes
        .iter()
        .find(|n| n.id == id)
        .expect("layout of a known node");
    let w = node.title.chars().count() + 4;
    let extra = if cross_targets(graph, id).is_empty() {
        0
    } else {
        1
    };
    (w, 3 + extra)
}

fn child_ids(graph: &PlanGraph, id: u64) -> Vec<u64> {
    children(graph, id).iter().map(|n| n.id).collect()
}

fn subtree_width(graph: &PlanGraph, id: u64) -> usize {
    let (w, _) = box_size(graph, id);
    let kids = child_ids(graph, id);
    if kids.is_empty() {
        return w;
    }
    let total: usize = kids.iter().map(|k| subtree_width(graph, *k)).sum();
    (total + CHILD_GAP * (kids.len() - 1)).max(w)
}

/// Place `id`'s box so its subtree occupies `slot_left..slot_left+subtree`,
/// centered in the slot; children form a centered group below.
fn place(
    graph: &PlanGraph,
    id: u64,
    top: usize,
    slot_left: isize,
    out: &mut Vec<(u64, isize, usize, usize, usize)>,
) {
    let (w, h) = box_size(graph, id);
    let slot_w = subtree_width(graph, id) as isize;
    let bx = slot_left + (slot_w - w as isize) / 2;
    out.push((id, bx, top, w, h));
    let kids = child_ids(graph, id);
    if kids.is_empty() {
        return;
    }
    let group: usize = kids
        .iter()
        .map(|k| subtree_width(graph, *k))
        .sum::<usize>()
        + CHILD_GAP * (kids.len() - 1);
    let cx = bx + w as isize / 2;
    let mut x = cx - group as isize / 2;
    for k in kids {
        place(graph, k, top + h + LINK_H, x, out);
        x += subtree_width(graph, k) as isize + CHILD_GAP as isize;
    }
}

/// Lay out every node top-down. Returns rects plus total canvas size.
pub fn layout_boxes(graph: &PlanGraph) -> (Vec<BoxRect>, usize, usize) {
    let mut raw: Vec<(u64, isize, usize, usize, usize)> = Vec::new();
    let mut cursor_y = 0usize;
    for root in roots(graph) {
        let before = raw.len();
        place(graph, root.id, cursor_y, 0, &mut raw);
        let bottom = raw[before..].iter().map(|(_, _, y, _, h)| y + h).max().unwrap_or(cursor_y);
        cursor_y = bottom + 2;
    }
    let min_x = raw.iter().map(|(_, x, _, _, _)| *x).min().unwrap_or(0).min(0);
    let rects: Vec<BoxRect> = raw
        .into_iter()
        .map(|(id, x, y, w, h)| BoxRect {
            id,
            x: (x - min_x) as usize,
            y,
            w,
            h,
        })
        .collect();
    let total_w = rects.iter().map(|r| r.x + r.w).max().unwrap_or(0);
    let total_h = rects.iter().map(|r| r.y + r.h).max().unwrap_or(0);
    (rects, total_w, total_h)
}

fn rect_of(layout: &[BoxRect], id: u64) -> &BoxRect {
    layout.iter().find(|r| r.id == id).expect("rect for known node")
}

/// Char grid cell: character plus its style.
pub type Cell = (char, Style);

fn set(grid: &mut [Vec<Cell>], x: usize, y: usize, ch: char, style: Style) {
    if let Some(row) = grid.get_mut(y) {
        if let Some(cell) = row.get_mut(x) {
            *cell = (ch, style);
        }
    }
}

/// Draw boxes, ports and elbow connectors onto a char grid.
pub fn draw_canvas(
    graph: &PlanGraph,
    layout: &[BoxRect],
    selected: Option<u64>,
    total_w: usize,
    total_h: usize,
) -> Vec<Vec<Cell>> {
    let mut grid = vec![vec![(' ', Style::default()); total_w]; total_h];
    // Connectors first, boxes paint over junctions.
    for node in &graph.nodes {
        let kids = child_ids(graph, node.id);
        if kids.is_empty() {
            continue;
        }
        let p = rect_of(layout, node.id);
        let cx_p = p.x + p.w / 2;
        let stem_y = p.y + p.h;
        if kids.len() == 1 {
            let c = rect_of(layout, kids[0]);
            let cx = c.x + c.w / 2;
            debug_assert_eq!(cx, cx_p);
            for y in stem_y..stem_y + LINK_H {
                set(&mut grid, cx, y, '│', Style::default());
            }
            continue;
        }
        set(&mut grid, cx_p, stem_y, '│', Style::default());
        let bus = stem_y + 1;
        let centers: Vec<usize> = kids
            .iter()
            .map(|k| {
                let c = rect_of(layout, *k);
                c.x + c.w / 2
            })
            .collect();
        let min_cx = *centers.iter().min().unwrap();
        let max_cx = *centers.iter().max().unwrap();
        for x in min_cx..=max_cx {
            set(&mut grid, x, bus, '─', Style::default());
        }
        for cx in &centers {
            set(&mut grid, *cx, bus, '┬', Style::default());
        }
        if centers.contains(&cx_p) {
            set(&mut grid, cx_p, bus, '┼', Style::default());
        } else {
            set(&mut grid, cx_p, bus, '┴', Style::default());
        }
        for cx in &centers {
            set(&mut grid, *cx, bus + 1, '│', Style::default());
        }
    }
    // Boxes with rounded corners, centered ports, optional cross-link row.
    for r in layout {
        let node = graph
            .nodes
            .iter()
            .find(|n| n.id == r.id)
            .expect("node for known rect");
        let style = if selected == Some(r.id) {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        let cx = r.x + r.w / 2 - r.x; // center offset inside the box
        for (i, y) in (r.y..r.y + r.h).enumerate() {
            match i {
                0 => {
                    for dx in 0..r.w {
                        let ch = if dx == 0 {
                            '╭'
                        } else if dx == r.w - 1 {
                            '╮'
                        } else if dx == cx {
                            '┬'
                        } else {
                            '─'
                        };
                        set(&mut grid, r.x + dx, y, ch, style);
                    }
                }
                1 => {
                    set(&mut grid, r.x, y, '│', style);
                    set(&mut grid, r.x + 1, y, ' ', style);
                    for (j, ch) in node.title.chars().enumerate() {
                        set(&mut grid, r.x + 2 + j, y, ch, style);
                    }
                    set(&mut grid, r.x + 2 + node.title.chars().count(), y, ' ', style);
                    set(&mut grid, r.x + r.w - 1, y, '│', style);
                }
                _ if i == r.h - 1 => {
                    for dx in 0..r.w {
                        let ch = if dx == 0 {
                            '╰'
                        } else if dx == r.w - 1 {
                            '╯'
                        } else if dx == cx {
                            '┴'
                        } else {
                            '─'
                        };
                        set(&mut grid, r.x + dx, y, ch, style);
                    }
                }
                _ => {
                    // Cross-link row: `⇢ targets`, truncated to inner width.
                    let inner = r.w - 2;
                    let text: String = format!("⇢ {}", cross_targets(graph, r.id).join(", "));
                    let mut shown: String = text.chars().take(inner).collect();
                    while shown.chars().count() < inner {
                        shown.push(' ');
                    }
                    set(&mut grid, r.x, y, '│', style);
                    for (j, ch) in shown.chars().enumerate() {
                        set(&mut grid, r.x + 1 + j, y, ch, style);
                    }
                    set(&mut grid, r.x + r.w - 1, y, '│', style);
                }
            }
        }
    }
    grid
}

/// Slice a viewport out of the grid, merging same-style runs into spans.
pub fn canvas_view(
    grid: &[Vec<Cell>],
    w: usize,
    h: usize,
    row_off: usize,
    col_off: usize,
) -> Vec<Line<'static>> {
    grid.iter()
        .skip(row_off)
        .take(h)
        .map(|row| {
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut buf = String::new();
            let mut cur: Option<Style> = None;
            let flush = |buf: &mut String, cur: &mut Option<Style>, spans: &mut Vec<Span<'static>>| {
                if !buf.is_empty() {
                    spans.push(Span::styled(
                        std::mem::take(buf),
                        cur.unwrap_or_default(),
                    ));
                }
            };
            for (ch, st) in row.iter().skip(col_off).take(w) {
                if cur != Some(*st) {
                    flush(&mut buf, &mut cur, &mut spans);
                    cur = Some(*st);
                }
                buf.push(*ch);
            }
            flush(&mut buf, &mut cur, &mut spans);
            Line::from(spans)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Screen: Flow (boxes) left, tree Outline right.
// ---------------------------------------------------------------------------

pub fn render_plan(f: &mut ratatui::Frame, pm: &mut crate::plan::PlanMode, area: Rect) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::widgets::{Block, Borders, Paragraph};

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Flow pane.
    {
        let c = chunks[0];
        pm.flow_view_w = c.width.saturating_sub(2) as usize;
        pm.flow_view_h = c.height.saturating_sub(2) as usize;
        let (layout, total_w, total_h) = layout_boxes(&pm.graph);
        pm.follow_flow(&layout, total_w, total_h);
        let grid = draw_canvas(&pm.graph, &layout, pm.selected_node, total_w, total_h);
        let lines = canvas_view(&grid, pm.flow_view_w, pm.flow_view_h, pm.flow_row, pm.flow_col);
        let widget =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Flow Chart"));
        f.render_widget(widget, c);
    }

    // Outline pane (tree style).
    {
        let c = chunks[1];
        pm.outline_view_h = c.height.saturating_sub(2) as usize;
        pm.follow_outline();
        let tree = render_tree(&pm.graph);
        let lines: Vec<Line> = tree
            .into_iter()
            .skip(pm.outline_scroll)
            .map(|(id, line)| {
                if pm.selected_node == Some(id) {
                    let text: String = line.to_string();
                    Line::from(Span::styled(text, Style::default().add_modifier(Modifier::REVERSED)))
                } else {
                    line
                }
            })
            .collect();
        let widget =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Outline"));
        f.render_widget(widget, c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(lines: &[(u64, Line)]) -> Vec<String> {
        lines.iter().map(|(_, l)| l.to_string()).collect()
    }

    #[test]
    fn render_tree_shows_hierarchy() {
        let mut g = PlanGraph::new();
        let root = g.create_node("Root", None).unwrap();
        g.create_node("Child A", Some(root)).unwrap();
        g.create_node("Child B", Some(root)).unwrap();
        let text = text_of(&render_tree(&g));
        assert!(text.iter().any(|l| l == "Root"), "lines: {text:?}");
        assert!(text.iter().any(|l| l == "├─ Child A"), "lines: {text:?}");
        assert!(text.iter().any(|l| l == "└─ Child B"), "lines: {text:?}");
    }

    #[test]
    fn render_tree_nests_grandchildren() {
        let mut g = PlanGraph::new();
        let root = g.create_node("Root", None).unwrap();
        let child = g.create_node("Child", Some(root)).unwrap();
        g.create_node("Grandchild", Some(child)).unwrap();
        let text = text_of(&render_tree(&g));
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
    fn render_tree_marks_cross_link_target() {
        let mut g = PlanGraph::new();
        let a = g.create_node("Alpha", None).unwrap();
        let b = g.create_node("Beta", None).unwrap();
        g.connect_create(a, b).unwrap();
        let text = text_of(&render_tree(&g));
        assert!(
            text.iter().any(|l| l == "Alpha ⇢ Beta"),
            "lines: {text:?}"
        );
    }

    #[test]
    fn render_tree_empty_graph() {
        let g = PlanGraph::new();
        assert!(render_tree(&g).is_empty());
    }

    #[test]
    fn render_tree_carries_node_ids() {
        let mut g = PlanGraph::new();
        let root = g.create_node("Root", None).unwrap();
        let child = g.create_node("Child", Some(root)).unwrap();
        let ids: Vec<u64> = render_tree(&g).iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![root, child]);
    }

    #[test]
    fn layout_centers_children_under_parent() {
        let mut g = PlanGraph::new();
        let root = g.create_node("Root", None).unwrap();
        let a = g.create_node("A", Some(root)).unwrap();
        let b = g.create_node("B", Some(root)).unwrap();
        let (layout, total_w, total_h) = layout_boxes(&g);
        let r = |id| *layout.iter().find(|r| r.id == id).unwrap();
        // Root box is 4 + 4 = 8 wide; children A/B are 1 + 4 = 5 wide each.
        assert_eq!((r(root).w, r(a).w, r(b).w), (8, 5, 5));
        // Children sit one LINK_H below the root box bottom.
        assert_eq!(r(a).y, r(root).y + r(root).h + LINK_H);
        assert_eq!(r(b).y, r(a).y);
        // Group centered under the root center: root center 4, group 5+4+5=14 → starts at -3 → shifted to 0.
        assert_eq!(r(a).x, 0);
        assert_eq!(r(b).x, 9);
        assert_eq!(r(root).x, 3);
        assert_eq!(total_w, 14);
        assert_eq!(total_h, r(b).y + r(b).h);
    }

    #[test]
    fn layout_single_child_is_straight() {
        let mut g = PlanGraph::new();
        let root = g.create_node("Root", None).unwrap();
        let child = g.create_node("Child", Some(root)).unwrap();
        let (layout, _, _) = layout_boxes(&g);
        let r = |id| *layout.iter().find(|r| r.id == id).unwrap();
        let cx = |id| r(id).x + r(id).w / 2;
        assert_eq!(cx(root), cx(child));
    }

    #[test]
    fn layout_cross_link_grows_box() {
        let mut g = PlanGraph::new();
        let a = g.create_node("A", None).unwrap();
        let b = g.create_node("B", None).unwrap();
        g.connect_create(a, b).unwrap();
        let (layout, _, _) = layout_boxes(&g);
        let r = |id| *layout.iter().find(|r| r.id == id).unwrap();
        assert_eq!(r(a).h, 4);
        assert_eq!(r(b).h, 3);
    }

    fn grid_text(grid: &[Vec<Cell>]) -> Vec<String> {
        grid.iter()
            .map(|row| row.iter().map(|(ch, _)| *ch).collect())
            .collect()
    }

    #[test]
    fn draw_single_chain_is_vertical() {
        let mut g = PlanGraph::new();
        let root = g.create_node("R", None).unwrap();
        g.create_node("C", Some(root)).unwrap();
        let (layout, w, h) = layout_boxes(&g);
        let grid = draw_canvas(&g, &layout, None, w, h);
        let text = grid_text(&grid);
        // Root box: ╭─┬─╮ / │ R │ / ╰─┴─╯ ; straight │ stem; child box below.
        assert!(text[0].contains("╭─┬─╮"), "grid: {text:?}");
        assert!(text[1].contains("│ R │"), "grid: {text:?}");
        assert!(text.iter().any(|l| l.contains("│ C │")), "grid: {text:?}");
        let stem_row = 3;
        assert_eq!(text[stem_row].chars().nth(2), Some('│'), "grid: {text:?}");
    }

    #[test]
    fn draw_sibling_bus_has_elbows() {
        let mut g = PlanGraph::new();
        let root = g.create_node("Root", None).unwrap();
        g.create_node("A", Some(root)).unwrap();
        g.create_node("B", Some(root)).unwrap();
        let (layout, w, h) = layout_boxes(&g);
        let grid = draw_canvas(&g, &layout, None, w, h);
        let text = grid_text(&grid);
        let bus = text
            .iter()
            .find(|l| l.contains('┬') || l.contains('┼'))
            .cloned()
            .expect("bus row");
        assert!(bus.contains('─'), "bus: {bus}");
    }

    #[test]
    fn draw_selected_box_is_reversed() {
        let mut g = PlanGraph::new();
        let root = g.create_node("R", None).unwrap();
        let (layout, w, h) = layout_boxes(&g);
        let grid = draw_canvas(&g, &layout, Some(root), w, h);
        assert!(
            grid.iter().flatten().all(|(_, st)| st
                == &Style::default().add_modifier(Modifier::REVERSED)),
            "every cell of the single box must be reversed"
        );
        let plain = draw_canvas(&g, &layout, None, w, h);
        assert!(
            plain
                .iter()
                .flatten()
                .all(|(_, st)| *st == Style::default())
        );
    }

    #[test]
    fn canvas_view_slices_and_merges_spans() {
        let grid = vec![
            vec![('a', Style::default()), ('b', Style::default())],
            vec![('c', Style::default()), ('d', Style::default())],
        ];
        let lines = canvas_view(&grid, 10, 10, 1, 1);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "d");
        // Empty viewport.
        assert!(canvas_view(&grid, 10, 0, 0, 0).is_empty());
    }
}
