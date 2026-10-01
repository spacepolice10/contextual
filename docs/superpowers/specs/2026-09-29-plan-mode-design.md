# Plan Mode Design

**Date:** 2026-09-29
**Status:** Draft

## Overview

Plan mode is a third screen in the `contextual` TUI application (alongside Picker and Viewer). It provides a mind-map-style planning tool with hierarchical nodes, optional cross-links, and dual-pane read-only display (flow-chart + outline). All mutations happen through commands (CLI or MCP); the UI is read-only.

## Architecture

```
src/
  plan/
    mod.rs       — PlanMode struct, screen switching, initialization
    model.rs     — PlanGraph, PlanNode, PlanLink (core data + command execution)
    render.rs    — flow-chart + outline rendering
    commands.rs  — command parsing and dispatch (shared by CLI and MCP)
```

### Module Responsibilities

- **`model.rs`** — Pure data layer. `PlanGraph` holds nodes and links. All command logic lives here as methods. No I/O, no rendering. Fully testable.
- **`commands.rs`** — Parses command strings into `PlanCommand` enum, dispatches to `model.rs`. Used by both CLI and MCP.
- **`render.rs`** — Takes `PlanGraph` and produces ratatui widgets for both panes. No mutation.
- **`mod.rs`** — `PlanMode` struct integrating model + render + input handling. Manages file loading/saving.

## Data Model

### JSON Format

```json
{
  "nodes": [
    { "id": 1, "title": "Root idea", "parent": null },
    { "id": 2, "title": "Child A", "parent": 1 },
    { "id": 3, "title": "Cross-link target", "parent": null }
  ],
  "links": [
    { "from": 2, "to": 3, "type": "cross" }
  ]
}
```

### Rust Types

```rust
pub struct PlanNode {
    pub id: u64,
    pub title: String,
    pub parent: Option<u64>,
}

pub struct PlanLink {
    pub from: u64,
    pub to: u64,
    pub link_type: LinkType,
}

pub enum LinkType {
    Hierarchy,  // parent-child (derived from PlanNode.parent, stored implicitly)
    Cross,      // arbitrary connection (escape hatch)
}

pub struct PlanGraph {
    pub nodes: Vec<PlanNode>,
    pub cross_links: Vec<PlanLink>,
    pub next_id: u64,
}
```

### Storage

- Default path: `~/.contextual/plan.json`
- Override: `--plan-file <path>` CLI flag
- Auto-save after every mutation
- Pretty-printed JSON (git-friendly)

## Commands

All commands are available via CLI and MCP with identical semantics.

### Plan File Commands

| Command | Args | Description |
|---------|------|-------------|
| `plan_create` | `<path>` | Create new empty plan file and open it |
| `plan_open` | `<path>` | Open existing plan file |

### Node Commands

| Command | Args | Description |
|---------|------|-------------|
| `node_create` | `<title> [parent_id]` | Create node (optionally as child of parent_id) |
| `node_update` | `<id> <new_title>` | Rename node |
| `node_delete` | `<id>` | Delete node and its subtree |
| `node_list` | — | List all nodes |
| `node_show` | `<id>` | Show node details |

### Link Commands

| Command | Args | Description |
|---------|------|-------------|
| `link_create` | `<parent_id> <child_id>` | Create hierarchical link |
| `link_remove` | `<parent_id> <child_id>` | Remove hierarchical link |
| `connect_create` | `<from_id> <to_id>` | Create arbitrary cross-link |
| `connect_remove` | `<from_id> <to_id>` | Remove arbitrary cross-link |

## UI Layout

### Screen Structure

```
┌─────────────────────────────────────────────────┐
│  Plan Mode                          [file: ...] │
├───────────────────────────────┬─────────────────┤
│                               │                 │
│      Flow-Chart Pane          │   Outline Pane  │
│      (Unicode tree)           │   (indented)    │
│                               │                 │
│                               │                 │
├───────────────────────────────┴─────────────────┤
│  Commands: plan_new, node_create, link_create...│
└─────────────────────────────────────────────────┘
```

### Flow-Chart Pane (Left)

- Unicode box-drawing characters for tree structure
- Indentation shows hierarchy
- Cross-links shown with dashed lines or markers
- Selected node highlighted (selection only, no editing)

### Outline Pane (Right)

- Flat list with indentation for hierarchy
- Cross-links marked with `→ target_id` suffix
- Scrollable independently

### Read-Only Constraint

- No keyboard editing in UI
- Selection allowed (for reference in commands)
- All mutations through CLI/MCP commands

## CLI Integration

### Clap Subcommands

```bash
contextual plan --file <path>              # Open plan mode
contextual plan_create <path>               # Create new plan file
contextual plan_open <path>                # Open existing plan file
contextual node_create "Title" [parent]    # Create node
contextual node_update <id> "New title"    # Rename node
contextual node_delete <id>                # Delete node
contextual node_list                       # List nodes
contextual node_show <id>                  # Show node
contextual link_create <parent> <child>    # Hierarchical link
contextual link_remove <parent> <child>    # Remove link
contextual connect_create <from> <to>      # Cross-link
contextual connect_remove <from> <to>      # Remove cross-link
```

### Picker Integration

- Picker shows `.plan.json` files (or any `.json` with valid plan structure)
- Selecting a plan file opens Plan mode
- "New plan" action in picker creates file and opens it

## MCP Integration

Same commands exposed as MCP tools:

- `plan_new`, `plan_open`
- `node_create`, `node_update`, `node_delete`, `node_list`, `node_show`
- `link_create`, `link_remove`, `connect_create`, `connect_remove`

## Error Handling

- Invalid node ID → error message
- Duplicate link → error message
- File not found → error message
- Invalid JSON → error message with parse details
- All errors returned as `Result<T, PlanError>`

## Testing

- Unit tests for all `model.rs` commands
- Integration tests for CLI parsing
- JSON round-trip tests (serialize → deserialize → compare)
