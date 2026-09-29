mod app;
mod cli;
mod highlight;
mod input;
mod picker;
mod plan;
mod search;
mod select;
mod tui;
mod ui;
mod viewer;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::event::{self, Event};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    // Handle plan subcommands before entering TUI
    if let Some(subcmd) = &cli.plan {
        return handle_plan_command(subcmd, cli.plan_file.as_deref());
    }

    let mut app = match cli.path {
        Some(p) => app::App::load_file(&p, false)?,
        None => {
            let (files, truncated) = picker::discover_files(std::path::Path::new("."));
            app::App::new_picker(files, truncated)
        }
    };
    let _guard = tui::TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut term = Terminal::new(backend).context("create terminal")?;
    loop {
        term.draw(|f| render(f, &mut app))?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(k) = event::read()? {
                // Estimate of visible text rows: total height minus 2 status
                // rows (line + bottom padding) minus 2 body border rows.
                app.viewport_h = term.size()?.height.saturating_sub(4) as usize;
                if input::handle(&mut app, k.code, k.modifiers)? {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn handle_plan_command(
    subcmd: &cli::PlanSubcommand,
    plan_file: Option<&std::path::Path>,
) -> Result<()> {
    use crate::plan::commands::{execute_command, PlanCommand};
    use crate::plan::model::PlanGraph;
    use cli::PlanSubcommand;

    let default_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("contextual")
        .join("plan.json");
    let path = plan_file.unwrap_or(&default_path);

    let mut graph = PlanGraph::load(path).unwrap_or_else(|_| PlanGraph::new());

    let cmd = match subcmd {
        PlanSubcommand::Plan => {
            // Open plan mode in TUI — handled by caller
            return Ok(());
        }
        PlanSubcommand::PlanCreate { path } => {
            let new_graph = PlanGraph::new();
            new_graph.save(path)?;
            println!("Created new plan: {}", path.display());
            return Ok(());
        }
        PlanSubcommand::PlanOpen { path } => {
            let loaded = PlanGraph::load(path)?;
            println!("Opened plan: {}", path.display());
            println!("Nodes: {}", loaded.nodes.len());
            return Ok(());
        }
        PlanSubcommand::NodeCreate { title, parent_id } => PlanCommand::NodeCreate {
            title: title.clone(),
            parent_id: *parent_id,
        },
        PlanSubcommand::NodeUpdate { id, new_title } => PlanCommand::NodeUpdate {
            id: *id,
            new_title: new_title.clone(),
        },
        PlanSubcommand::NodeDelete { id } => PlanCommand::NodeDelete { id: *id },
        PlanSubcommand::NodeList => PlanCommand::NodeList,
        PlanSubcommand::NodeShow { id } => PlanCommand::NodeShow { id: *id },
        PlanSubcommand::LinkCreate { parent_id, child_id } => PlanCommand::LinkCreate {
            parent_id: *parent_id,
            child_id: *child_id,
        },
        PlanSubcommand::LinkRemove { parent_id, child_id } => PlanCommand::LinkRemove {
            parent_id: *parent_id,
            child_id: *child_id,
        },
        PlanSubcommand::ConnectCreate { from_id, to_id } => PlanCommand::ConnectCreate {
            from_id: *from_id,
            to_id: *to_id,
        },
        PlanSubcommand::ConnectRemove { from_id, to_id } => PlanCommand::ConnectRemove {
            from_id: *from_id,
            to_id: *to_id,
        },
    };

    let result = execute_command(&mut graph, &cmd)?;
    println!("{}", result);
    graph.save(path)?;
    Ok(())
}

fn render(f: &mut ratatui::Frame, app: &mut app::App) {
    use app::Mode;
    let area = f.area();
    match app.mode {
        Mode::Picker => crate::ui::picker::render_picker(f, app, area),
        Mode::Viewer => crate::ui::viewer::render_viewer(f, app, area),
        Mode::Plan => {
            if let Some(pm) = &app.plan {
                crate::plan::render_plan(f, &pm.graph, area);
            }
        }
    }
}
