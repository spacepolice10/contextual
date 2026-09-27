use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "contextual", about = "Minimal terminal text viewer")]
pub struct Cli {
    /// File to open. If omitted, shows picker for current directory.
    pub path: Option<PathBuf>,
}
