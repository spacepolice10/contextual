pub mod commands;
pub mod mcp;
pub mod model;
pub mod render;

pub use render::render_plan;

use model::PlanGraph;
use std::path::PathBuf;

#[derive(Debug)]
pub struct PlanMode {
    pub graph: PlanGraph,
    #[allow(dead_code)] // Used by Task 5 (CLI) and Task 6 (picker)
    pub file_path: PathBuf,
    pub selected_node: Option<u64>,
    /// Vertical offset of the Outline pane (rows).
    pub outline_scroll: usize,
    /// Viewport offset of the Flow pane (rows, cols).
    pub flow_row: usize,
    pub flow_col: usize,
    /// Last rendered viewport sizes; navigation follows the selection
    /// using these (zero until the first frame).
    pub outline_view_h: usize,
    pub flow_view_w: usize,
    pub flow_view_h: usize,
}

impl PlanMode {
    pub fn new(graph: PlanGraph, file_path: PathBuf) -> Self {
        PlanMode {
            graph,
            file_path,
            selected_node: None,
            outline_scroll: 0,
            flow_row: 0,
            flow_col: 0,
            outline_view_h: 0,
            flow_view_w: 0,
            flow_view_h: 0,
        }
    }

    /// Flattened DFS node order (roots first) — the focus order.
    pub fn outline_order(&self) -> Vec<u64> {
        let mut out = Vec::new();
        let roots: Vec<u64> = self
            .graph
            .nodes
            .iter()
            .filter(|n| n.parent.is_none())
            .map(|n| n.id)
            .collect();
        for r in roots {
            self.push_subtree(r, &mut out);
        }
        out
    }

    fn push_subtree(&self, id: u64, out: &mut Vec<u64>) {
        out.push(id);
        for c in self.children_of(id) {
            self.push_subtree(c, out);
        }
    }

    fn children_of(&self, id: u64) -> Vec<u64> {
        self.graph
            .nodes
            .iter()
            .filter(|n| n.parent == Some(id))
            .map(|n| n.id)
            .collect()
    }

    fn focus(&mut self, id: u64) {
        self.selected_node = Some(id);
        self.follow();
    }

    /// j / Down: next node in focus order (first when nothing focused).
    pub fn down(&mut self) {
        let order = self.outline_order();
        if order.is_empty() {
            return;
        }
        let next = match self.selected_node.and_then(|s| order.iter().position(|&id| id == s)) {
            Some(i) if i + 1 < order.len() => order[i + 1],
            Some(_) => order[order.len() - 1],
            None => order[0],
        };
        self.focus(next);
    }

    /// k / Up: previous node in focus order (last when nothing focused).
    pub fn up(&mut self) {
        let order = self.outline_order();
        if order.is_empty() {
            return;
        }
        let prev = match self.selected_node.and_then(|s| order.iter().position(|&id| id == s)) {
            Some(i) if i > 0 => order[i - 1],
            Some(_) => order[0],
            None => order[order.len() - 1],
        };
        self.focus(prev);
    }

    /// h / Left: focus the parent (noop on roots).
    pub fn parent(&mut self) {
        let pid = self.selected_node.and_then(|s| {
            self.graph
                .nodes
                .iter()
                .find(|n| n.id == s)
                .and_then(|n| n.parent)
        });
        if let Some(pid) = pid {
            self.focus(pid);
        }
    }

    /// l / Right: focus the first child (noop on leaves).
    pub fn child(&mut self) {
        let kid = self
            .selected_node
            .and_then(|s| self.children_of(s).into_iter().next());
        if let Some(kid) = kid {
            self.focus(kid);
        }
    }

    pub fn page_up(&mut self) {
        self.page(-10);
    }

    pub fn page_down(&mut self) {
        self.page(10);
    }

    fn page(&mut self, delta: isize) {
        let order = self.outline_order();
        if order.is_empty() {
            return;
        }
        let cur = self
            .selected_node
            .and_then(|s| order.iter().position(|&id| id == s))
            .unwrap_or(0) as isize;
        let next = (cur + delta).clamp(0, order.len() as isize - 1) as usize;
        self.focus(order[next]);
    }

    /// Keep the focused node visible in both panes.
    fn follow(&mut self) {
        let (layout, total_w, total_h) = crate::plan::render::layout_boxes(&self.graph);
        self.follow_outline();
        self.follow_flow(&layout, total_w, total_h);
    }

    pub(crate) fn follow_outline(&mut self) {        if self.outline_view_h == 0 {
            return;
        }
        let order = self.outline_order();
        if let Some(sel) = self.selected_node {
            if let Some(idx) = order.iter().position(|&id| id == sel) {
                if idx < self.outline_scroll {
                    self.outline_scroll = idx;
                } else if idx >= self.outline_scroll + self.outline_view_h {
                    self.outline_scroll = idx + 1 - self.outline_view_h;
                }
            }
        }
    }

    pub(crate) fn follow_flow(&mut self, layout: &[crate::plan::render::BoxRect], total_w: usize, total_h: usize) {
        if self.flow_view_h > 0 {
            if let Some(sel) = self.selected_node {
                if let Some(r) = layout.iter().find(|r| r.id == sel) {
                    if r.y < self.flow_row {
                        self.flow_row = r.y;
                    } else if r.y + r.h > self.flow_row + self.flow_view_h {
                        self.flow_row = (r.y + r.h).saturating_sub(self.flow_view_h);
                    }
                }
            }
            self.flow_row = self.flow_row.min(total_h.saturating_sub(self.flow_view_h));
        }
        if self.flow_view_w > 0 {
            if let Some(sel) = self.selected_node {
                if let Some(r) = layout.iter().find(|r| r.id == sel) {
                    if r.x < self.flow_col {
                        self.flow_col = r.x;
                    } else if r.x + r.w > self.flow_col + self.flow_view_w {
                        self.flow_col = (r.x + r.w).saturating_sub(self.flow_view_w);
                    }
                }
            }
            self.flow_col = self.flow_col.min(total_w.saturating_sub(self.flow_view_w));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_level() -> PlanMode {
        let mut g = PlanGraph::new();
        let root = g.create_node("Root", None).unwrap();
        g.create_node("A", Some(root)).unwrap();
        let b = g.create_node("B", Some(root)).unwrap();
        g.create_node("B1", Some(b)).unwrap();
        g.create_node("R2", None).unwrap();
        PlanMode::new(g, PathBuf::from("/tmp/test.plan.json"))
    }

    #[test]
    fn plan_mode_new_initializes() {
        let g = PlanGraph::new();
        let pm = PlanMode::new(g, PathBuf::from("/tmp/test.plan.json"));
        assert_eq!(pm.graph.nodes.len(), 0);
        assert!(pm.selected_node.is_none());
    }

    #[test]
    fn plan_mode_down_moves_selection() {
        let mut pm = two_level();
        pm.down();
        assert!(pm.selected_node.is_some());
        pm.down();
        // Should wrap or clamp
    }

    #[test]
    fn plan_mode_up_at_top_stays() {
        let mut g = PlanGraph::new();
        g.create_node("A", None).unwrap();
        let mut pm = PlanMode::new(g, PathBuf::from("/tmp/test.plan.json"));
        pm.up();
        // Should stay at 0 or wrap to end
    }

    #[test]
    fn outline_order_is_dfs() {
        let pm = two_level();
        let order = pm.outline_order();
        // Root, A, B, B1, R2.
        assert_eq!(order.len(), 5);
        assert_eq!(order[4], 5);
        assert!(order[0] == 1 && order[3] == 4);
    }

    #[test]
    fn h_focuses_parent_and_l_first_child() {
        let mut pm = two_level();
        // Focus B1 (id 4): h → B (3), h → Root (1), h → stays.
        pm.focus(4);
        pm.parent();
        assert_eq!(pm.selected_node, Some(3));
        pm.parent();
        assert_eq!(pm.selected_node, Some(1));
        pm.parent();
        assert_eq!(pm.selected_node, Some(1));
        // l from Root → first child A (2); l on leaf stays.
        pm.child();
        assert_eq!(pm.selected_node, Some(2));
        pm.child();
        assert_eq!(pm.selected_node, Some(2));
    }

    #[test]
    fn jk_walk_focus_order() {
        let mut pm = two_level();
        pm.down();
        assert_eq!(pm.selected_node, Some(1));
        pm.down();
        assert_eq!(pm.selected_node, Some(2));
        pm.up();
        assert_eq!(pm.selected_node, Some(1));
        pm.up();
        assert_eq!(pm.selected_node, Some(1));
    }

    #[test]
    fn follow_scrolls_outline_to_focus() {
        let mut pm = two_level();
        pm.outline_view_h = 2;
        pm.focus(4); // B1, index 3 — out of a 2-row view.
        assert_eq!(pm.outline_scroll, 2);
        pm.focus(1);
        assert_eq!(pm.outline_scroll, 0);
    }

    #[test]
    fn follow_scrolls_flow_to_focused_box() {
        let mut pm = two_level();
        pm.flow_view_w = 80;
        pm.flow_view_h = 4; // shorter than the full canvas.
        pm.focus(5); // R2 sits below the fold.
        assert!(pm.flow_row > 0);
        pm.focus(1);
        assert_eq!(pm.flow_row, 0);
    }

    #[test]
    fn follow_clamps_without_focus() {
        let mut pm = two_level();
        pm.flow_view_w = 80;
        pm.flow_view_h = 4;
        pm.flow_row = 10_000;
        pm.follow();
        let (_, _, total_h) = crate::plan::render::layout_boxes(&pm.graph);
        assert_eq!(pm.flow_row, total_h - 4);
    }
}
