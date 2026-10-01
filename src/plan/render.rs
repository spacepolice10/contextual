use crate::highlight::Theme;
use crate::plan::model::PlanGraph;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
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

/// Depth-tinted backgrounds, cycling through a small palette.
/// Dark themes get dark slate tones, light themes pastels; the default
/// foreground is untouched so text stays readable on both.
const DARK_BG: &[Color] = &[
    Color::Rgb(38, 40, 64),
    Color::Rgb(30, 48, 66),
    Color::Rgb(52, 36, 66),
    Color::Rgb(30, 54, 54),
];
const LIGHT_BG: &[Color] = &[
    Color::Rgb(232, 234, 246),
    Color::Rgb(226, 238, 244),
    Color::Rgb(240, 230, 246),
    Color::Rgb(230, 244, 238),
];

pub fn depth_style(depth: usize, theme: Theme) -> Style {
    Style::default().bg(depth_color(depth, theme))
}

fn depth_color(depth: usize, theme: Theme) -> Color {
    let palette = match theme {
        Theme::Dark => DARK_BG,
        Theme::Light => LIGHT_BG,
    };
    palette[depth % palette.len()]
}

fn shift_channel(v: u8, delta: isize) -> u8 {
    (v as isize + delta).clamp(0, 255) as u8
}

/// Frame around a block: the same depth hue, pushed brighter on dark
/// themes and deeper on light ones so the ring reads against both the
/// block and the terminal background.
pub fn frame_style(depth: usize, theme: Theme) -> Style {
    let delta = match theme {
        Theme::Dark => 40,
        Theme::Light => -36,
    };
    let color = match depth_color(depth, theme) {
        Color::Rgb(r, g, b) => Color::Rgb(
            shift_channel(r, delta),
            shift_channel(g, delta),
            shift_channel(b, delta),
        ),
        other => other,
    };
    Style::default().bg(color)
}

/// Depth of a node (roots are 0), walking parent pointers.
/// Bounded by the node count so hand-edited cyclic data terminates.
fn node_depth(graph: &PlanGraph, id: u64) -> usize {
    let mut depth = 0;
    let mut cur = id;
    for _ in 0..graph.nodes.len() + 1 {
        match graph.nodes.iter().find(|n| n.id == cur).and_then(|n| n.parent) {
            Some(parent) => {
                depth += 1;
                cur = parent;
            }
            None => break,
        }
    }
    depth
}

/// Draw boxes, ports and elbow connectors onto a char grid.
pub fn draw_canvas(
    graph: &PlanGraph,
    layout: &[BoxRect],
    selected: Option<u64>,
    total_w: usize,
    total_h: usize,
    theme: Theme,
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
        // Rounded ends at the outer drops; the parent stem junction keeps
        // its ┼/┴ shape (a 3-way joint has no rounded form).
        for cx in &centers {
            let ch = if *cx == min_cx {
                '╭'
            } else if *cx == max_cx {
                '╮'
            } else {
                '┬'
            };
            set(&mut grid, *cx, bus, ch, Style::default());
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
    // Frameless blocks: a 1-cell frame ring in the bright variant of the
    // depth color, content rows on the depth background. Connector lines
    // touch the outer frame directly, so there are no port glyphs.
    for r in layout {
        let node = graph
            .nodes
            .iter()
            .find(|n| n.id == r.id)
            .expect("node for known rect");
        let depth = node_depth(graph, r.id);
        let mut content = depth_style(depth, theme);
        let mut frame = frame_style(depth, theme);
        if selected == Some(r.id) {
            content = content.add_modifier(Modifier::REVERSED);
            frame = frame.add_modifier(Modifier::REVERSED);
        }
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                if y == r.y || y == r.y + r.h - 1 || x == r.x || x == r.x + r.w - 1 {
                    set(&mut grid, x, y, ' ', frame);
                }
            }
        }
        // One inner text row: left-padded, space-filled to the inner width.
        let put_row = |grid: &mut Vec<Vec<Cell>>, y: usize, text: &str| {
            let inner = r.w - 2;
            let mut shown = format!(" {text}");
            while shown.chars().count() < inner {
                shown.push(' ');
            }
            let shown: String = shown.chars().take(inner).collect();
            for (j, ch) in shown.chars().enumerate() {
                set(grid, r.x + 1 + j, y, ch, content);
            }
        };
        let mut row = r.y + 1;
        put_row(&mut grid, row, &node.title);
        row += 1;
        if row < r.y + r.h - 1 {
            // Cross-link row: `⇢ targets`.
            let text = format!("⇢ {}", cross_targets(graph, r.id).join(", "));
            put_row(&mut grid, row, &text);
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

pub fn render_plan(
    f: &mut ratatui::Frame,
    pm: &mut crate::plan::PlanMode,
    area: Rect,
    theme: Theme,
) {
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
        let grid = draw_canvas(&pm.graph, &layout, pm.selected_node, total_w, total_h, theme);
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
        let grid = draw_canvas(&g, &layout, None, w, h, crate::highlight::Theme::Dark);
        let text = grid_text(&grid);
        // Frameless blocks: title rows, straight │ stem between the blocks.
        assert!(text.iter().any(|l| l.contains('R')), "grid: {text:?}");
        assert!(text.iter().any(|l| l.contains('C')), "grid: {text:?}");
        assert!(
            !text.iter().any(|l| l.contains('╭') || l.contains('╰')),
            "no thin borders: {text:?}"
        );
        let r = layout.iter().find(|r| r.id == root).unwrap();
        // Stem starts at the outer frame bottom and runs to the child frame.
        let stem_row = r.y + r.h;
        assert_eq!(text[stem_row].chars().nth(2), Some('│'), "grid: {text:?}");
    }

    #[test]
    fn frame_style_is_brighter_variant_of_depth() {
        use crate::highlight::Theme;
        fn rgb(s: ratatui::style::Style) -> (u8, u8, u8) {
            match s.bg {
                Some(ratatui::style::Color::Rgb(r, g, b)) => (r, g, b),
                other => panic!("expected rgb bg, got {other:?}"),
            }
        }
        // Dark theme: frame channels are lifted.
        let (br, bg, bb) = rgb(depth_style(0, Theme::Dark));
        let (fr, fg, fb) = rgb(frame_style(0, Theme::Dark));
        assert!(fr >= br && fg >= bg && fb >= bb);
        assert!((fr, fg, fb) != (br, bg, bb));
        // Light theme: frame channels are deepened for contrast.
        let (br, bg, bb) = rgb(depth_style(1, Theme::Light));
        let (fr, fg, fb) = rgb(frame_style(1, Theme::Light));
        assert!(fr <= br && fg <= bg && fb <= bb);
        assert!((fr, fg, fb) != (br, bg, bb));
    }

    #[test]
    fn draw_block_has_frame_ring_and_content() {
        use crate::highlight::Theme;
        let mut g = PlanGraph::new();
        let root = g.create_node("R", None).unwrap();
        let (layout, w, h) = layout_boxes(&g);
        let grid = draw_canvas(&g, &layout, None, w, h, Theme::Dark);
        let r = layout.iter().find(|r| r.id == root).unwrap();
        // Outer corner = frame style, space glyph.
        assert_eq!(grid[r.y][r.x], (' ', frame_style(0, Theme::Dark)));
        // Title cell = depth background.
        assert_eq!(grid[r.y + 1][r.x + 1].1, depth_style(0, Theme::Dark));
    }

    #[test]
    fn draw_sibling_bus_has_rounded_ends() {
        let mut g = PlanGraph::new();
        let root = g.create_node("Root", None).unwrap();
        g.create_node("A", Some(root)).unwrap();
        g.create_node("B", Some(root)).unwrap();
        let (layout, w, h) = layout_boxes(&g);
        let grid = draw_canvas(&g, &layout, None, w, h, crate::highlight::Theme::Dark);
        let text = grid_text(&grid);
        // Bus row: rounded ends at the outer drops, ┴ where the parent
        // stem (centered between the children) meets the bus.
        // (Box borders also contain ╭…╮, so pin the bus by its ┴ junction.)
        let bus = text
            .iter()
            .find(|l| l.contains('┴') && !l.contains('╰'))
            .cloned()
            .expect("rounded bus row");
        assert!(bus.contains('╭'), "bus: {bus}");
        assert!(bus.contains('╮'), "bus: {bus}");
        assert!(bus.contains('─'), "bus: {bus}");
    }

    #[test]
    fn depth_style_differs_by_depth_and_theme() {
        use crate::highlight::Theme;
        let d0 = depth_style(0, Theme::Dark);
        let d1 = depth_style(1, Theme::Dark);
        let l0 = depth_style(0, Theme::Light);
        assert_ne!(d0, d1);
        assert_ne!(d0, l0);
        // Cycles back through the palette instead of growing forever.
        assert_eq!(depth_style(0, Theme::Dark), depth_style(4, Theme::Dark));
    }

    #[test]
    fn draw_box_has_depth_background() {
        use crate::highlight::Theme;
        let mut g = PlanGraph::new();
        let root = g.create_node("Root", None).unwrap();
        let child = g.create_node("Child", Some(root)).unwrap();
        let (layout, w, h) = layout_boxes(&g);
        let grid = draw_canvas(&g, &layout, None, w, h, Theme::Dark);
        let r = |id| *layout.iter().find(|r| r.id == id).unwrap();
        // Outer corners = bright frame; title cells = depth background.
        assert_eq!(grid[r(root).y][r(root).x].1, frame_style(0, Theme::Dark));
        assert_eq!(grid[r(root).y + 1][r(root).x + 1].1, depth_style(0, Theme::Dark));
        assert_eq!(grid[r(child).y + 1][r(child).x + 1].1, depth_style(1, Theme::Dark));
    }

    #[test]
    fn draw_selected_box_is_reversed() {
        use crate::highlight::Theme;
        let mut g = PlanGraph::new();
        let root = g.create_node("R", None).unwrap();
        let (layout, w, h) = layout_boxes(&g);
        let r = layout.iter().find(|r| r.id == root).unwrap();
        let grid = draw_canvas(&g, &layout, Some(root), w, h, Theme::Dark);
        // Frame ring and content both carry REVERSED on top of their bg.
        let frame = frame_style(0, Theme::Dark).add_modifier(Modifier::REVERSED);
        let base = depth_style(0, Theme::Dark).add_modifier(Modifier::REVERSED);
        assert_eq!(grid[r.y][r.x].1, frame);
        assert_eq!(grid[r.y + 1][r.x + 1].1, base);
        let plain = draw_canvas(&g, &layout, None, w, h, Theme::Dark);
        assert_eq!(plain[r.y][r.x].1, frame_style(0, Theme::Dark));
        assert_eq!(plain[r.y + 1][r.x + 1].1, depth_style(0, Theme::Dark));
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
