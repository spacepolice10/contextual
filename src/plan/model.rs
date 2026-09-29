use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNode {
    pub id: u64,
    pub title: String,
    pub parent: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanLink {
    pub from: u64,
    pub to: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanGraph {
    pub nodes: Vec<PlanNode>,
    pub cross_links: Vec<PlanLink>,
    pub next_id: u64,
}

#[derive(Debug)]
pub enum PlanError {
    NodeNotFound(u64),
    DuplicateLink,
    InvalidParent(u64),
    // Constructed by MCP arg parsing (staged module); allow until transport lands.
    #[allow(dead_code)]
    InvalidArgs(String),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::NodeNotFound(id) => write!(f, "Node {} not found", id),
            PlanError::DuplicateLink => write!(f, "Link already exists"),
            PlanError::InvalidParent(id) => write!(f, "Parent node {} not found", id),
            PlanError::InvalidArgs(msg) => write!(f, "Invalid arguments: {}", msg),
            PlanError::Io(e) => write!(f, "IO error: {}", e),
            PlanError::Json(e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl std::error::Error for PlanError {}

impl PlanGraph {
    pub fn new() -> Self {
        PlanGraph {
            nodes: Vec::new(),
            cross_links: Vec::new(),
            next_id: 1,
        }
    }

    pub fn create_node(&mut self, title: &str, parent: Option<u64>) -> Result<u64, PlanError> {
        if let Some(pid) = parent {
            if !self.nodes.iter().any(|n| n.id == pid) {
                return Err(PlanError::InvalidParent(pid));
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(PlanNode {
            id,
            title: title.to_string(),
            parent,
        });
        Ok(id)
    }

    pub fn update_node(&mut self, id: u64, new_title: &str) -> Result<(), PlanError> {
        let node = self
            .nodes
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or(PlanError::NodeNotFound(id))?;
        node.title = new_title.to_string();
        Ok(())
    }

    pub fn delete_node(&mut self, id: u64) -> Result<(), PlanError> {
        if !self.nodes.iter().any(|n| n.id == id) {
            return Err(PlanError::NodeNotFound(id));
        }
        // Collect all descendants recursively
        let mut to_delete = vec![id];
        let mut i = 0;
        while i < to_delete.len() {
            let current = to_delete[i];
            for node in &self.nodes {
                if node.parent == Some(current) {
                    to_delete.push(node.id);
                }
            }
            i += 1;
        }
        self.nodes.retain(|n| !to_delete.contains(&n.id));
        self.cross_links.retain(|l| !to_delete.contains(&l.from) && !to_delete.contains(&l.to));
        Ok(())
    }

    #[allow(dead_code)] // Used by commands.rs (Task 2)
    pub fn list_nodes(&self) -> &[PlanNode] {
        &self.nodes
    }

    #[allow(dead_code)] // Used by commands.rs (Task 2)
    pub fn show_node(&self, id: u64) -> Result<&PlanNode, PlanError> {
        self.nodes
            .iter()
            .find(|n| n.id == id)
            .ok_or(PlanError::NodeNotFound(id))
    }

    pub fn link_create(&mut self, parent_id: u64, child_id: u64) -> Result<(), PlanError> {
        if !self.nodes.iter().any(|n| n.id == parent_id) {
            return Err(PlanError::NodeNotFound(parent_id));
        }
        if !self.nodes.iter().any(|n| n.id == child_id) {
            return Err(PlanError::NodeNotFound(child_id));
        }
        if self.nodes.iter().any(|n| n.id == child_id && n.parent == Some(parent_id)) {
            return Err(PlanError::DuplicateLink);
        }
        let child = self
            .nodes
            .iter_mut()
            .find(|n| n.id == child_id)
            .unwrap();
        child.parent = Some(parent_id);
        Ok(())
    }

    pub fn link_remove(&mut self, parent_id: u64, child_id: u64) -> Result<(), PlanError> {
        let child = self
            .nodes
            .iter_mut()
            .find(|n| n.id == child_id)
            .ok_or(PlanError::NodeNotFound(child_id))?;
        if child.parent == Some(parent_id) {
            child.parent = None;
        }
        Ok(())
    }

    pub fn connect_create(&mut self, from_id: u64, to_id: u64) -> Result<(), PlanError> {
        if !self.nodes.iter().any(|n| n.id == from_id) {
            return Err(PlanError::NodeNotFound(from_id));
        }
        if !self.nodes.iter().any(|n| n.id == to_id) {
            return Err(PlanError::NodeNotFound(to_id));
        }
        if self.cross_links.iter().any(|l| l.from == from_id && l.to == to_id) {
            return Err(PlanError::DuplicateLink);
        }
        self.cross_links.push(PlanLink {
            from: from_id,
            to: to_id,
        });
        Ok(())
    }

    pub fn connect_remove(&mut self, from_id: u64, to_id: u64) -> Result<(), PlanError> {
        let before = self.cross_links.len();
        self.cross_links.retain(|l| !(l.from == from_id && l.to == to_id));
        if self.cross_links.len() == before {
            return Err(PlanError::NodeNotFound(from_id));
        }
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<(), PlanError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(PlanError::Json)?;
        std::fs::write(path, json).map_err(PlanError::Io)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, PlanError> {
        let content = std::fs::read_to_string(path).map_err(PlanError::Io)?;
        serde_json::from_str(&content).map_err(PlanError::Json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_node_assigns_incrementing_ids() {
        let mut g = PlanGraph::new();
        let id1 = g.create_node("Root", None).unwrap();
        let id2 = g.create_node("Child", Some(id1)).unwrap();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.nodes[1].parent, Some(id1));
    }

    #[test]
    fn create_node_invalid_parent_errors() {
        let mut g = PlanGraph::new();
        let result = g.create_node("Orphan", Some(999));
        assert!(matches!(result, Err(PlanError::InvalidParent(999))));
    }

    #[test]
    fn update_node_renames() {
        let mut g = PlanGraph::new();
        let id = g.create_node("Old", None).unwrap();
        g.update_node(id, "New").unwrap();
        assert_eq!(g.nodes[0].title, "New");
    }

    #[test]
    fn update_node_not_found_errors() {
        let mut g = PlanGraph::new();
        let result = g.update_node(999, "New");
        assert!(matches!(result, Err(PlanError::NodeNotFound(999))));
    }

    #[test]
    fn delete_node_removes_subtree() {
        let mut g = PlanGraph::new();
        let root = g.create_node("Root", None).unwrap();
        let child = g.create_node("Child", Some(root)).unwrap();
        let _grandchild = g.create_node("Grandchild", Some(child)).unwrap();
        g.delete_node(root).unwrap();
        assert_eq!(g.nodes.len(), 0);
    }

    #[test]
    fn link_create_establishes_parent_child() {
        let mut g = PlanGraph::new();
        let parent = g.create_node("Parent", None).unwrap();
        let child = g.create_node("Child", None).unwrap();
        g.link_create(parent, child).unwrap();
        assert_eq!(g.nodes[1].parent, Some(parent));
    }

    #[test]
    fn link_create_duplicate_errors() {
        let mut g = PlanGraph::new();
        let parent = g.create_node("Parent", None).unwrap();
        let child = g.create_node("Child", None).unwrap();
        g.link_create(parent, child).unwrap();
        let result = g.link_create(parent, child);
        assert!(matches!(result, Err(PlanError::DuplicateLink)));
    }

    #[test]
    fn link_remove_clears_parent() {
        let mut g = PlanGraph::new();
        let parent = g.create_node("Parent", None).unwrap();
        let child = g.create_node("Child", None).unwrap();
        g.link_create(parent, child).unwrap();
        g.link_remove(parent, child).unwrap();
        assert_eq!(g.nodes[1].parent, None);
    }

    #[test]
    fn connect_create_adds_cross_link() {
        let mut g = PlanGraph::new();
        let a = g.create_node("A", None).unwrap();
        let b = g.create_node("B", None).unwrap();
        g.connect_create(a, b).unwrap();
        assert_eq!(g.cross_links.len(), 1);
        assert_eq!(g.cross_links[0].from, a);
        assert_eq!(g.cross_links[0].to, b);
    }

    #[test]
    fn connect_create_duplicate_errors() {
        let mut g = PlanGraph::new();
        let a = g.create_node("A", None).unwrap();
        let b = g.create_node("B", None).unwrap();
        g.connect_create(a, b).unwrap();
        let result = g.connect_create(a, b);
        assert!(matches!(result, Err(PlanError::DuplicateLink)));
    }

    #[test]
    fn connect_remove_deletes_cross_link() {
        let mut g = PlanGraph::new();
        let a = g.create_node("A", None).unwrap();
        let b = g.create_node("B", None).unwrap();
        g.connect_create(a, b).unwrap();
        g.connect_remove(a, b).unwrap();
        assert_eq!(g.cross_links.len(), 0);
    }

    #[test]
    fn save_load_round_trip() {
        let mut g = PlanGraph::new();
        let root = g.create_node("Root", None).unwrap();
        let child = g.create_node("Child", Some(root)).unwrap();
        let other = g.create_node("Other", None).unwrap();
        g.connect_create(child, other).unwrap();
        let dir = std::env::temp_dir();
        let path = dir.join("ctx_plan_test.json");
        g.save(&path).unwrap();
        let loaded = PlanGraph::load(&path).unwrap();
        assert_eq!(loaded.nodes.len(), 3);
        assert_eq!(loaded.nodes[0].title, "Root");
        assert_eq!(loaded.nodes[1].parent, Some(root));
        assert_eq!(loaded.cross_links.len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_invalid_json_errors() {
        let dir = std::env::temp_dir();
        let path = dir.join("ctx_plan_invalid.json");
        std::fs::write(&path, "not json").unwrap();
        let result = PlanGraph::load(&path);
        assert!(matches!(result, Err(PlanError::Json(_))));
        let _ = std::fs::remove_file(&path);
    }
}
