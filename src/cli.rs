use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "contextual", about = "Minimal terminal text viewer")]
pub struct Cli {
    /// File to open. If omitted, shows picker for current directory.
    pub path: Option<PathBuf>,

    /// Plan file to use (default: ~/.contextual/plan.json)
    #[arg(long, value_name = "PATH")]
    pub plan_file: Option<PathBuf>,

    /// Plan subcommands for managing plan files and nodes
    #[command(subcommand)]
    pub plan: Option<PlanSubcommand>,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "snake_case")]
pub enum PlanSubcommand {
    /// Open plan mode in TUI
    Plan,
    /// Create a new plan file
    PlanCreate {
        /// Path for the new plan file
        path: PathBuf,
    },
    /// Open an existing plan file
    PlanOpen {
        /// Path to the plan file
        path: PathBuf,
    },
    /// Create a new node
    NodeCreate {
        /// Node title
        title: String,
        /// Optional parent node ID
        parent_id: Option<u64>,
    },
    /// Update a node's title
    NodeUpdate {
        /// Node ID
        id: u64,
        /// New title
        new_title: String,
    },
    /// Delete a node and its subtree
    NodeDelete {
        /// Node ID
        id: u64,
    },
    /// List all nodes
    NodeList,
    /// Show node details
    NodeShow {
        /// Node ID
        id: u64,
    },
    /// Create a hierarchical link
    LinkCreate {
        /// Parent node ID
        parent_id: u64,
        /// Child node ID
        child_id: u64,
    },
    /// Remove a hierarchical link
    LinkRemove {
        /// Parent node ID
        parent_id: u64,
        /// Child node ID
        child_id: u64,
    },
    /// Create a cross-link between nodes
    ConnectCreate {
        /// Source node ID
        from_id: u64,
        /// Target node ID
        to_id: u64,
    },
    /// Remove a cross-link
    ConnectRemove {
        /// Source node ID
        from_id: u64,
        /// Target node ID
        to_id: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_plan_flag_parses() {
        let cli = Cli::parse_from(["contextual", "plan"]);
        assert!(cli.plan.is_some());
    }

    #[test]
    fn cli_plan_file_flag_parses() {
        let cli = Cli::parse_from(["contextual", "--plan-file", "/tmp/x.plan.json", "plan"]);
        assert_eq!(cli.plan_file, Some(PathBuf::from("/tmp/x.plan.json")));
        assert!(cli.plan.is_some());
    }

    #[test]
    fn cli_node_update_with_trailing_args() {
        let cli = Cli::parse_from(["contextual", "node_update", "1", "New Title"]);
        assert!(cli.plan.is_some());
    }

    #[test]
    fn cli_node_create_parses() {
        let cli = Cli::parse_from(["contextual", "node_create", "Hello"]);
        assert!(cli.plan.is_some());
    }

    #[test]
    fn cli_node_create_with_parent_parses() {
        let cli = Cli::parse_from(["contextual", "node_create", "Hello", "1"]);
        assert!(cli.plan.is_some());
    }

    #[test]
    fn cli_node_update_parses() {
        let cli = Cli::parse_from(["contextual", "node_update", "1", "NewTitle"]);
        assert!(cli.plan.is_some());
    }

    #[test]
    fn cli_node_delete_parses() {
        let cli = Cli::parse_from(["contextual", "node_delete", "1"]);
        assert!(cli.plan.is_some());
    }

    #[test]
    fn cli_node_list_parses() {
        let cli = Cli::parse_from(["contextual", "node_list"]);
        assert!(cli.plan.is_some());
    }

    #[test]
    fn cli_node_show_parses() {
        let cli = Cli::parse_from(["contextual", "node_show", "1"]);
        assert!(cli.plan.is_some());
    }

    #[test]
    fn cli_link_create_parses() {
        let cli = Cli::parse_from(["contextual", "link_create", "1", "2"]);
        assert!(cli.plan.is_some());
    }

    #[test]
    fn cli_link_remove_parses() {
        let cli = Cli::parse_from(["contextual", "link_remove", "1", "2"]);
        assert!(cli.plan.is_some());
    }

    #[test]
    fn cli_connect_create_parses() {
        let cli = Cli::parse_from(["contextual", "connect_create", "1", "2"]);
        assert!(cli.plan.is_some());
    }

    #[test]
    fn cli_connect_remove_parses() {
        let cli = Cli::parse_from(["contextual", "connect_remove", "1", "2"]);
        assert!(cli.plan.is_some());
    }

    #[test]
    fn cli_plan_create_parses() {
        let cli = Cli::parse_from(["contextual", "plan_create", "/tmp/test.plan.json"]);
        assert!(cli.plan.is_some());
    }

    #[test]
    fn cli_plan_open_parses() {
        let cli = Cli::parse_from(["contextual", "plan_open", "/tmp/test.plan.json"]);
        assert!(cli.plan.is_some());
    }
}
