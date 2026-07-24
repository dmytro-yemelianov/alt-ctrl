//! Rust terminal interface for Alt Ctrl.
//!
//! The current binary renders deterministic fixture sessions. It exercises the
//! shared navigation and safety contracts without launching agents or commands.

pub mod fixtures;
pub mod input;
pub mod model;
pub mod ui;

use std::{
    io::{self, Write},
    time::Duration,
};

use crossterm::{
    cursor::Show,
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{CrosstermBackend, TestBackend},
};

use crate::{fixtures::fixture_state, input::map_key_event, ui::render};

/// Runs the interactive fixture TUI until the user quits.
///
/// # Errors
///
/// Returns an I/O error if terminal setup, event reading, drawing, or terminal
/// restoration fails.
pub fn run() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    if let Err(error) = execute!(stdout, EnterAlternateScreen) {
        disable_raw_mode()?;
        return Err(error);
    }
    let mut restore_guard = TerminalRestoreGuard::new();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run_loop(&mut terminal);
    let restore_result = restore_terminal(&mut terminal);
    if restore_result.is_ok() {
        restore_guard.disarm();
    }

    result.and(restore_result)
}

/// Renders a deterministic, ANSI-free Mission Control frame.
///
/// # Errors
///
/// Returns an I/O error if the test backend cannot draw the frame.
pub fn snapshot(width: u16, height: u16) -> io::Result<String> {
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;
    let state = fixture_state();
    terminal.draw(|frame| render(frame, &state))?;
    Ok(snapshot_from_terminal(&terminal))
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut state = fixture_state();
    while !state.should_quit {
        terminal.draw(|frame| render(frame, &state))?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if let Some(command) = map_key_event(key) {
                    state.apply(&command);
                }
            }
        }
    }
    Ok(())
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;
    terminal.show_cursor()
}

struct TerminalRestoreGuard {
    armed: bool,
}

impl TerminalRestoreGuard {
    const fn new() -> Self {
        Self { armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TerminalRestoreGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
        }
    }
}

pub(crate) fn snapshot_from_terminal(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let mut output = String::new();
    for y in 0..buffer.area.height {
        let mut row = String::new();
        for x in 0..buffer.area.width {
            row.push_str(buffer[(x, y)].symbol());
        }
        output.push_str(row.trim_end());
        output.push('\n');
    }
    output
}

/// Writes a deterministic Mission Control snapshot to standard output.
///
/// # Errors
///
/// Returns an I/O error if rendering or writing fails.
pub fn print_snapshot(width: u16, height: u16) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(snapshot(width, height)?.as_bytes())
}
