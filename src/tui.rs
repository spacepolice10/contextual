use anyhow::{Context, Result};
use std::io;

pub struct TerminalGuard;
impl TerminalGuard {
    pub fn enter() -> Result<Self> {
        crossterm::terminal::enable_raw_mode().context("enable raw mode")?;
        let mut out = io::stdout();
        crossterm::execute!(out, crossterm::terminal::EnterAlternateScreen)
            .context("enter alt screen")?;
        Ok(Self)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen);
    }
}
