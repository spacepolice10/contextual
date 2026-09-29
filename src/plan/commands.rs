use crate::plan::model::{PlanError, PlanGraph};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum PlanCommand {
    // Constructed via parse_command by MCP integration (Task 7);
    // CLI (Task 5) handles file commands directly.
    #[allow(dead_code)]
    PlanCreate { path: PathBuf },
    #[allow(dead_code)]
    PlanOpen { path: PathBuf },
    NodeCreate { title: String, parent_id: Option<u64> },
    NodeUpdate { id: u64, new_title: String },
    NodeDelete { id: u64 },
    NodeList,
    NodeShow { id: u64 },
    LinkCreate { parent_id: u64, child_id: u64 },
    LinkRemove { parent_id: u64, child_id: u64 },
    ConnectCreate { from_id: u64, to_id: u64 },
    ConnectRemove { from_id: u64, to_id: u64 },
}

#[allow(dead_code)] // Entry point for MCP integration (Task 7)
pub fn parse_command(input: &str) -> Result<PlanCommand, String> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }
    match parts[0] {
        "plan_create" => {
            if parts.len() < 2 {
                return Err("plan_create requires <path>".to_string());
            }
            Ok(PlanCommand::PlanCreate {
                path: PathBuf::from(parts[1]),
            })
        }
        "plan_open" => {
            if parts.len() < 2 {
                return Err("plan_open requires <path>".to_string());
            }
            Ok(PlanCommand::PlanOpen {
                path: PathBuf::from(parts[1]),
            })
        }
        "node_create" => {
            if parts.len() < 2 {
                return Err("node_create requires <title>".to_string());
            }
            let title = parts[1].to_string();
            let parent_id = if parts.len() > 2 {
                Some(parts[2].parse::<u64>().map_err(|_| "Invalid parent_id")?)
            } else {
                None
            };
            Ok(PlanCommand::NodeCreate { title, parent_id })
        }
        "node_update" => {
            if parts.len() < 3 {
                return Err("node_update requires <id> <new_title>".to_string());
            }
            let id = parts[1].parse::<u64>().map_err(|_| "Invalid id")?;
            let new_title = parts[2].to_string();
            Ok(PlanCommand::NodeUpdate { id, new_title })
        }
        "node_delete" => {
            if parts.len() < 2 {
                return Err("node_delete requires <id>".to_string());
            }
            let id = parts[1].parse::<u64>().map_err(|_| "Invalid id")?;
            Ok(PlanCommand::NodeDelete { id })
        }
        "node_list" => Ok(PlanCommand::NodeList),
        "node_show" => {
            if parts.len() < 2 {
                return Err("node_show requires <id>".to_string());
            }
            let id = parts[1].parse::<u64>().map_err(|_| "Invalid id")?;
            Ok(PlanCommand::NodeShow { id })
        }
        "link_create" => {
            if parts.len() < 3 {
                return Err("link_create requires <parent_id> <child_id>".to_string());
            }
            let parent_id = parts[1].parse::<u64>().map_err(|_| "Invalid parent_id")?;
            let child_id = parts[2].parse::<u64>().map_err(|_| "Invalid child_id")?;
            Ok(PlanCommand::LinkCreate { parent_id, child_id })
        }
        "link_remove" => {
            if parts.len() < 3 {
                return Err("link_remove requires <parent_id> <child_id>".to_string());
            }
            let parent_id = parts[1].parse::<u64>().map_err(|_| "Invalid parent_id")?;
            let child_id = parts[2].parse::<u64>().map_err(|_| "Invalid child_id")?;
            Ok(PlanCommand::LinkRemove { parent_id, child_id })
        }
        "connect_create" => {
            if parts.len() < 3 {
                return Err("connect_create requires <from_id> <to_id>".to_string());
            }
            let from_id = parts[1].parse::<u64>().map_err(|_| "Invalid from_id")?;
            let to_id = parts[2].parse::<u64>().map_err(|_| "Invalid to_id")?;
            Ok(PlanCommand::ConnectCreate { from_id, to_id })
        }
        "connect_remove" => {
            if parts.len() < 3 {
                return Err("connect_remove requires <from_id> <to_id>".to_string());
            }
            let from_id = parts[1].parse::<u64>().map_err(|_| "Invalid from_id")?;
            let to_id = parts[2].parse::<u64>().map_err(|_| "Invalid to_id")?;
            Ok(PlanCommand::ConnectRemove { from_id, to_id })
        }
        _ => Err(format!("Unknown command: {}", parts[0])),
    }
}

/// True when executing `cmd` mutates the graph and the caller must persist.
/// Reads (`node_list`, `node_show`) and file commands (which manage their
/// own files) return false.
pub fn is_mutation(cmd: &PlanCommand) -> bool {
    matches!(
        cmd,
        PlanCommand::NodeCreate { .. }
            | PlanCommand::NodeUpdate { .. }
            | PlanCommand::NodeDelete { .. }
            | PlanCommand::LinkCreate { .. }
            | PlanCommand::LinkRemove { .. }
            | PlanCommand::ConnectCreate { .. }
            | PlanCommand::ConnectRemove { .. }
    )
}

pub fn execute_command(graph: &mut PlanGraph, cmd: &PlanCommand) -> Result<String, PlanError> {    match cmd {
        PlanCommand::PlanCreate { path } => {
            let new_graph = PlanGraph::new();
            new_graph.save(path)?;
            Ok(format!("Created new plan: {}", path.display()))
        }
        PlanCommand::PlanOpen { path } => {
            let loaded = PlanGraph::load(path)?;
            *graph = loaded;
            Ok(format!("Opened plan: {}", path.display()))
        }
        PlanCommand::NodeCreate { title, parent_id } => {
            let id = graph.create_node(title, *parent_id)?;
            Ok(format!("Created node {}: {}", id, title))
        }
        PlanCommand::NodeUpdate { id, new_title } => {
            graph.update_node(*id, new_title)?;
            Ok(format!("Updated node {}: {}", id, new_title))
        }
        PlanCommand::NodeDelete { id } => {
            graph.delete_node(*id)?;
            Ok(format!("Deleted node {}", id))
        }
        PlanCommand::NodeList => {
            if graph.nodes.is_empty() {
                return Ok("No nodes".to_string());
            }
            let mut out = String::new();
            for node in &graph.nodes {
                out.push_str(&format!("[{}] {}\n", node.id, node.title));
            }
            Ok(out.trim_end().to_string())
        }
        PlanCommand::NodeShow { id } => {
            let node = graph.show_node(*id)?;
            let parent_str = match node.parent {
                Some(p) => format!("parent: {}", p),
                None => "root".to_string(),
            };
            Ok(format!("id: {}\ntitle: {}\n{}", node.id, node.title, parent_str))
        }
        PlanCommand::LinkCreate { parent_id, child_id } => {
            graph.link_create(*parent_id, *child_id)?;
            Ok(format!("Linked {} -> {}", parent_id, child_id))
        }
        PlanCommand::LinkRemove { parent_id, child_id } => {
            graph.link_remove(*parent_id, *child_id)?;
            Ok(format!("Unlinked {} -> {}", parent_id, child_id))
        }
        PlanCommand::ConnectCreate { from_id, to_id } => {
            graph.connect_create(*from_id, *to_id)?;
            Ok(format!("Connected {} -> {}", from_id, to_id))
        }
        PlanCommand::ConnectRemove { from_id, to_id } => {
            graph.connect_remove(*from_id, *to_id)?;
            Ok(format!("Disconnected {} -> {}", from_id, to_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plan_create() {
        let cmd = parse_command("plan_create /tmp/test.plan.json").unwrap();
        assert!(matches!(cmd, PlanCommand::PlanCreate { path } if path == PathBuf::from("/tmp/test.plan.json")));
    }

    #[test]
    fn parse_node_create_no_parent() {
        let cmd = parse_command("node_create Hello").unwrap();
        assert!(matches!(cmd, PlanCommand::NodeCreate { title, parent_id: None } if title == "Hello"));
    }

    #[test]
    fn parse_node_create_with_parent() {
        let cmd = parse_command("node_create Hello 1").unwrap();
        assert!(matches!(cmd, PlanCommand::NodeCreate { title, parent_id: Some(1) } if title == "Hello"));
    }

    #[test]
    fn parse_node_update() {
        let cmd = parse_command("node_update 1 NewTitle").unwrap();
        assert!(matches!(cmd, PlanCommand::NodeUpdate { id: 1, new_title } if new_title == "NewTitle"));
    }

    #[test]
    fn parse_node_delete() {
        let cmd = parse_command("node_delete 1").unwrap();
        assert!(matches!(cmd, PlanCommand::NodeDelete { id: 1 }));
    }

    #[test]
    fn parse_node_list() {
        let cmd = parse_command("node_list").unwrap();
        assert!(matches!(cmd, PlanCommand::NodeList));
    }

    #[test]
    fn parse_node_show() {
        let cmd = parse_command("node_show 1").unwrap();
        assert!(matches!(cmd, PlanCommand::NodeShow { id: 1 }));
    }

    #[test]
    fn parse_link_create() {
        let cmd = parse_command("link_create 1 2").unwrap();
        assert!(matches!(cmd, PlanCommand::LinkCreate { parent_id: 1, child_id: 2 }));
    }

    #[test]
    fn parse_link_remove() {
        let cmd = parse_command("link_remove 1 2").unwrap();
        assert!(matches!(cmd, PlanCommand::LinkRemove { parent_id: 1, child_id: 2 }));
    }

    #[test]
    fn parse_connect_create() {
        let cmd = parse_command("connect_create 1 2").unwrap();
        assert!(matches!(cmd, PlanCommand::ConnectCreate { from_id: 1, to_id: 2 }));
    }

    #[test]
    fn parse_connect_remove() {
        let cmd = parse_command("connect_remove 1 2").unwrap();
        assert!(matches!(cmd, PlanCommand::ConnectRemove { from_id: 1, to_id: 2 }));
    }

    #[test]
    fn parse_unknown_command_errors() {
        let result = parse_command("unknown_cmd");
        assert!(result.is_err());
    }

    #[test]
    fn parse_missing_args_errors() {
        assert!(parse_command("node_create").is_err());
        assert!(parse_command("node_update 1").is_err());
        assert!(parse_command("link_create 1").is_err());
    }

    #[test]
    fn execute_node_create_returns_id() {
        let mut g = PlanGraph::new();
        let cmd = PlanCommand::NodeCreate { title: "Test".to_string(), parent_id: None };
        let result = execute_command(&mut g, &cmd).unwrap();
        assert!(result.contains("1"));
    }

    #[test]
    fn execute_node_list_shows_all() {
        let mut g = PlanGraph::new();
        g.create_node("A", None).unwrap();
        g.create_node("B", None).unwrap();
        let cmd = PlanCommand::NodeList;
        let result = execute_command(&mut g, &cmd).unwrap();
        assert!(result.contains("A"));
        assert!(result.contains("B"));
    }

    #[test]
    fn execute_node_show_displays_details() {
        let mut g = PlanGraph::new();
        let id = g.create_node("MyNode", None).unwrap();
        let cmd = PlanCommand::NodeShow { id };
        let result = execute_command(&mut g, &cmd).unwrap();
        assert!(result.contains("MyNode"));
        assert!(result.contains("id: 1"));
    }

    #[test]
    fn is_mutation_classifies_commands() {
        use std::path::PathBuf;
        // Reads never persist.
        assert!(!is_mutation(&PlanCommand::NodeList));
        assert!(!is_mutation(&PlanCommand::NodeShow { id: 1 }));
        // File commands manage their own files.
        assert!(!is_mutation(&PlanCommand::PlanCreate {
            path: PathBuf::from("x")
        }));
        assert!(!is_mutation(&PlanCommand::PlanOpen {
            path: PathBuf::from("x")
        }));
        // Every graph mutation persists.
        assert!(is_mutation(&PlanCommand::NodeCreate {
            title: "t".to_string(),
            parent_id: None
        }));
        assert!(is_mutation(&PlanCommand::NodeUpdate {
            id: 1,
            new_title: "t".to_string()
        }));
        assert!(is_mutation(&PlanCommand::NodeDelete { id: 1 }));
        assert!(is_mutation(&PlanCommand::LinkCreate {
            parent_id: 1,
            child_id: 2
        }));
        assert!(is_mutation(&PlanCommand::LinkRemove {
            parent_id: 1,
            child_id: 2
        }));
        assert!(is_mutation(&PlanCommand::ConnectCreate {
            from_id: 1,
            to_id: 2
        }));
        assert!(is_mutation(&PlanCommand::ConnectRemove {
            from_id: 1,
            to_id: 2
        }));
    }
}
