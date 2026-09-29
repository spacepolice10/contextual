// MCP transport is not wired yet (no server loop in this binary);
// the tool surface below is exercised by tests until then.
// Allow dead code until then so `cargo clippy -- -D warnings` stays clean.
#![allow(dead_code)]

use crate::plan::commands::{execute_command, PlanCommand};
use crate::plan::model::{PlanError, PlanGraph};

/// One MCP tool: name, human description, and JSON-schema input shape.
#[derive(Debug, Clone)]
pub struct McpTool {
    pub name: &'static str,
    pub description: &'static str,
    pub schema: serde_json::Value,
}

fn str_arg(args: &serde_json::Value, key: &str) -> Result<String, PlanError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| PlanError::InvalidArgs(format!("missing string arg `{key}`")))
}

fn u64_arg(args: &serde_json::Value, key: &str) -> Result<u64, PlanError> {
    args.get(key)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| PlanError::InvalidArgs(format!("missing u64 arg `{key}`")))
}

fn opt_u64_arg(args: &serde_json::Value, key: &str) -> Result<Option<u64>, PlanError> {
    match args.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => v.as_u64().map(Some).ok_or_else(|| {
            PlanError::InvalidArgs(format!("arg `{key}` must be a u64 or null"))
        }),
    }
}

fn tool(name: &'static str, description: &'static str, props: &[&'static str]) -> McpTool {
    let properties: serde_json::Map<String, serde_json::Value> = props
        .iter()
        .map(|p| (p.to_string(), serde_json::json!({"type": "string"})))
        .collect();
    McpTool {
        name,
        description,
        schema: serde_json::json!({"type": "object", "properties": properties}),
    }
}

pub fn mcp_tools() -> Vec<McpTool> {
    vec![
        tool("plan_create", "Create a new empty plan file", &["path"]),
        tool("plan_open", "Open an existing plan file", &["path"]),
        tool("node_create", "Create a node, optionally as a child", &["title", "parent_id"]),
        tool("node_update", "Rename a node", &["id", "new_title"]),
        tool("node_delete", "Delete a node and its subtree", &["id"]),
        tool("node_list", "List all nodes", &[]),
        tool("node_show", "Show node details", &["id"]),
        tool(
            "link_create",
            "Create a hierarchical parent-child link",
            &["parent_id", "child_id"],
        ),
        tool(
            "link_remove",
            "Remove a hierarchical parent-child link",
            &["parent_id", "child_id"],
        ),
        tool(
            "connect_create",
            "Create an arbitrary cross-link between nodes",
            &["from_id", "to_id"],
        ),
        tool(
            "connect_remove",
            "Remove an arbitrary cross-link between nodes",
            &["from_id", "to_id"],
        ),
    ]
}

pub fn handle_mcp_tool_with_graph(
    graph: &mut PlanGraph,
    name: &str,
    args: serde_json::Value,
) -> Result<String, PlanError> {
    use std::path::PathBuf;
    let cmd = match name {
        "plan_create" => PlanCommand::PlanCreate {
            path: PathBuf::from(str_arg(&args, "path")?),
        },
        "plan_open" => PlanCommand::PlanOpen {
            path: PathBuf::from(str_arg(&args, "path")?),
        },
        "node_create" => PlanCommand::NodeCreate {
            title: str_arg(&args, "title")?,
            parent_id: opt_u64_arg(&args, "parent_id")?,
        },
        "node_update" => PlanCommand::NodeUpdate {
            id: u64_arg(&args, "id")?,
            new_title: str_arg(&args, "new_title")?,
        },
        "node_delete" => PlanCommand::NodeDelete {
            id: u64_arg(&args, "id")?,
        },
        "node_list" => PlanCommand::NodeList,
        "node_show" => PlanCommand::NodeShow {
            id: u64_arg(&args, "id")?,
        },
        "link_create" => PlanCommand::LinkCreate {
            parent_id: u64_arg(&args, "parent_id")?,
            child_id: u64_arg(&args, "child_id")?,
        },
        "link_remove" => PlanCommand::LinkRemove {
            parent_id: u64_arg(&args, "parent_id")?,
            child_id: u64_arg(&args, "child_id")?,
        },
        "connect_create" => PlanCommand::ConnectCreate {
            from_id: u64_arg(&args, "from_id")?,
            to_id: u64_arg(&args, "to_id")?,
        },
        "connect_remove" => PlanCommand::ConnectRemove {
            from_id: u64_arg(&args, "from_id")?,
            to_id: u64_arg(&args, "to_id")?,
        },
        _ => return Err(PlanError::InvalidArgs(format!("unknown tool `{name}`"))),
    };
    execute_command(graph, &cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_tools_list_all_commands() {
        let tools = mcp_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name).collect();
        for expected in [
            "plan_create",
            "plan_open",
            "node_create",
            "node_update",
            "node_delete",
            "node_list",
            "node_show",
            "link_create",
            "link_remove",
            "connect_create",
            "connect_remove",
        ] {
            assert!(names.contains(&expected), "missing tool: {expected}");
        }
        assert_eq!(tools.len(), 11);
    }

    #[test]
    fn mcp_handle_node_create() {
        let mut g = PlanGraph::new();
        let args = serde_json::json!({"title": "MCP Node"});
        let result = handle_mcp_tool_with_graph(&mut g, "node_create", args).unwrap();
        assert!(result.contains("1"));
        assert_eq!(g.nodes.len(), 1);
    }

    #[test]
    fn mcp_handle_node_create_with_parent() {
        let mut g = PlanGraph::new();
        let parent = g.create_node("Parent", None).unwrap();
        let args = serde_json::json!({"title": "Child", "parent_id": parent});
        let result = handle_mcp_tool_with_graph(&mut g, "node_create", args).unwrap();
        assert!(result.contains("Child"));
        assert_eq!(g.nodes[1].parent, Some(parent));
    }

    #[test]
    fn mcp_handle_node_list() {
        let mut g = PlanGraph::new();
        g.create_node("A", None).unwrap();
        let result = handle_mcp_tool_with_graph(&mut g, "node_list", serde_json::json!({})).unwrap();
        assert!(result.contains("A"));
    }

    #[test]
    fn mcp_handle_unknown_tool_errors() {
        let mut g = PlanGraph::new();
        let result = handle_mcp_tool_with_graph(&mut g, "unknown_tool", serde_json::json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn mcp_handle_missing_args_errors() {
        let mut g = PlanGraph::new();
        // node_create without title
        let result = handle_mcp_tool_with_graph(&mut g, "node_create", serde_json::json!({}));
        assert!(matches!(result, Err(PlanError::InvalidArgs(_))));
        // node_show with non-numeric id
        let result = handle_mcp_tool_with_graph(
            &mut g,
            "node_show",
            serde_json::json!({"id": "abc"}),
        );
        assert!(matches!(result, Err(PlanError::InvalidArgs(_))));
    }

    #[test]
    fn mcp_handle_plan_and_link_tools() {
        let dir = std::env::temp_dir();
        let path = dir.join("ctx_mcp_test.plan.json");
        let _ = std::fs::remove_file(&path);
        let mut g = PlanGraph::new();
        let args = serde_json::json!({"path": path.to_string_lossy()});
        handle_mcp_tool_with_graph(&mut g, "plan_create", args).unwrap();
        assert!(path.exists());
        handle_mcp_tool_with_graph(&mut g, "node_create", serde_json::json!({"title": "A"}))
            .unwrap();
        handle_mcp_tool_with_graph(&mut g, "node_create", serde_json::json!({"title": "B"}))
            .unwrap();
        handle_mcp_tool_with_graph(
            &mut g,
            "link_create",
            serde_json::json!({"parent_id": 1, "child_id": 2}),
        )
        .unwrap();
        assert_eq!(g.nodes[1].parent, Some(1));
        handle_mcp_tool_with_graph(
            &mut g,
            "connect_create",
            serde_json::json!({"from_id": 2, "to_id": 1}),
        )
        .unwrap();
        assert_eq!(g.cross_links.len(), 1);
        let _ = std::fs::remove_file(&path);
    }
}
