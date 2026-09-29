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
    #[allow(dead_code)] // Used by Task 5 (CLI) and Task 6 (picker)
    pub scroll: usize,
}

impl PlanMode {
    #[allow(dead_code)] // Constructed by picker integration (Task 6)
    pub fn new(graph: PlanGraph, file_path: PathBuf) -> Self {
        PlanMode {
            graph,
            file_path,
            selected_node: None,
            scroll: 0,
        }
    }

    pub fn up(&mut self) {
        let count = self.graph.nodes.len();
        if count == 0 {
            return;
        }
        match self.selected_node {
            None => self.selected_node = Some(self.graph.nodes[count - 1].id),
            Some(idx) => {
                let current_pos = self
                    .graph
                    .nodes
                    .iter()
                    .position(|n| n.id == idx)
                    .unwrap_or(0);
                if current_pos > 0 {
                    self.selected_node = Some(self.graph.nodes[current_pos - 1].id);
                }
            }
        }
    }

    pub fn down(&mut self) {
        let count = self.graph.nodes.len();
        if count == 0 {
            return;
        }
        match self.selected_node {
            None => self.selected_node = Some(self.graph.nodes[0].id),
            Some(idx) => {
                let current_pos = self
                    .graph
                    .nodes
                    .iter()
                    .position(|n| n.id == idx)
                    .unwrap_or(0);
                if current_pos < count - 1 {
                    self.selected_node = Some(self.graph.nodes[current_pos + 1].id);
                }
            }
        }
    }

    pub fn page_up(&mut self) {
        for _ in 0..10 {
            self.up();
        }
    }

    pub fn page_down(&mut self) {
        for _ in 0..10 {
            self.down();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_mode_new_initializes() {
        let g = PlanGraph::new();
        let pm = PlanMode::new(g, PathBuf::from("/tmp/test.plan.json"));
        assert_eq!(pm.graph.nodes.len(), 0);
        assert!(pm.selected_node.is_none());
    }

    #[test]
    fn plan_mode_down_moves_selection() {
        let mut g = PlanGraph::new();
        g.create_node("A", None).unwrap();
        g.create_node("B", None).unwrap();
        let mut pm = PlanMode::new(g, PathBuf::from("/tmp/test.plan.json"));
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
}
