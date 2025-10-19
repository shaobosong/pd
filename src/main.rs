//! pd: An interactive parent directory navigator.
//!
//! This tool allows a user to interactively select a component of the current
//! working directory's path. It then prints the selected parent path to stdout,
//! which is intended to be captured by a shell function to quickly change
//! directories "up" the tree.
//!
//! # Features
//! - Vim and Emacs style keybindings.
//! - Mouse click and scroll wheel support for navigation.
//! - Cross-platform compatibility (Windows, macOS, Linux).
//! - Guaranteed terminal state restoration on exit via the RAII pattern.
//!
//! # Usage
//! The keymap can be set to Emacs mode by setting the `PD_KEYMAP` environment
//! variable to `emacs`. It defaults to Vim mode otherwise.

use std::{
    env,
    io::{stderr, Result, Write},
    path::{Component, Path, PathBuf},
};

use crossterm::{
    cursor,
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
        MouseEventKind,
    },
    execute,
    style::{Attribute, Print, SetAttribute},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};

// Conditionally compile the nix dependency only for unix targets.
#[cfg(unix)]
use nix::sys::signal::{self, Signal};

/// Puts the terminal into a "raw" mode.
///
/// This function enables raw mode, hides the cursor, and enables mouse capture.
/// This allows the application to have full control over terminal input and
/// display, rather than relying on line-buffered input.
fn set_terminal_mode() -> Result<()> {
    enable_raw_mode()?;
    execute!(stderr(), cursor::Hide, event::EnableMouseCapture)?;
    Ok(())
}

/// Restores the terminal to its normal state.
///
/// This function disables raw mode, shows the cursor, and disables mouse capture.
/// It also clears the screen from the cursor's position down to remove any UI artifacts.
fn restore_terminal_mode() -> Result<()> {
    // Failure to disable raw mode is usually safe to ignore, as the program is exiting.
    let _ = disable_raw_mode();
    let _ = execute!(stderr(), cursor::Show, event::DisableMouseCapture);
    let _ = execute!(
        stderr(),
        cursor::MoveToColumn(0),
        Clear(ClearType::FromCursorDown)
    );
    Ok(())
}

/// A guard struct to ensure terminal state is restored when it goes out of scope.
///
/// This struct leverages Rust's RAII (Resource Acquisition Is Initialization) pattern.
/// When an instance of `TermCleanup` is created, it does nothing. However, when it
/// falls out of scope (e.g., at the end of a function, on return, or during a panic),
/// its `drop` method is automatically called, which executes `restore_terminal_mode()`
/// to clean up the terminal state.
struct TermCleanup;

impl Drop for TermCleanup {
    fn drop(&mut self) {
        // Ensures that no matter how the program exits, the terminal mode is restored.
        let _ = restore_terminal_mode();
    }
}

/// Defines the supported keymap schemes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Keymap {
    /// Vim-style keybindings (h, j, k, l, etc.).
    Vim,
    /// Emacs-style keybindings (Ctrl-f, Ctrl-b, etc.).
    Emacs,
}

/// Holds the current state of the application.
///
/// This struct contains all the data necessary to drive the UI and logic.
struct AppState {
    /// A vector of strings representing the components of the current path.
    path_parts: Vec<String>,
    /// The index of the currently selected part in `path_parts`.
    current_index: usize,
    /// Stores numeric input for Vim-style count prefixes (e.g., `3h`).
    count_input: String,
}

impl AppState {
    /// Creates a new `AppState` from a vector of path components.
    ///
    /// By default, the last path component is selected.
    fn new(path_parts: Vec<String>) -> Self {
        let current_index = path_parts.len().saturating_sub(1);
        Self {
            path_parts,
            current_index,
            count_input: String::new(),
        }
    }

    /// Moves the selection index by a given step.
    ///
    /// `step` can be positive (move right) or negative (move left).
    /// The movement distance is multiplied by the number accumulated in `count_input`.
    /// The index is clamped to the valid range of `[0, path_parts.len() - 1]`.
    fn move_by(&mut self, step: isize) {
        let count = self.count_input.parse::<isize>().unwrap_or(1);
        self.current_index = (self.current_index as isize + step * count)
            .clamp(0, self.path_parts.len().saturating_sub(1) as isize) // Ensure it's within bounds
            as usize;
        self.count_input.clear(); // Reset count after movement
    }

    /// Moves the selection to the start of the path (the first component).
    fn move_to_start(&mut self) {
        self.current_index = 0;
        self.count_input.clear();
    }

    /// Moves the selection to the end of the path (the last component).
    fn move_to_end(&mut self) {
        self.current_index = self.path_parts.len().saturating_sub(1);
        self.count_input.clear();
    }

    /// Moves the selection to the middle of the path.
    fn move_to_middle(&mut self) {
        self.current_index = self.path_parts.len() / 2;
        self.count_input.clear();
    }

    /// Selects a path component based on the terminal column of a mouse click.
    ///
    /// This function iterates through the path parts, calculating their cumulative width,
    /// to determine which part covers the given `column`.
    fn select_part_at_column(&mut self, column: u16) {
        let mut current_pos: u16 = 0;
        for (i, part) in self.path_parts.iter().enumerate() {
            let part_len = part.chars().count() as u16;
            if column >= current_pos && column < current_pos + part_len {
                self.current_index = i;
                break;
            }
            current_pos += part_len;
        }
    }

    /// Constructs a `PathBuf` from the path components up to the current index.
    fn selected_path(&self) -> PathBuf {
        self.path_parts[..=self.current_index]
            .iter()
            .collect::<PathBuf>()
    }
}

/// Represents the possible actions resulting from event handling.
///
/// This enum defines the outcomes of processing user input, determining whether the
/// application should continue running, confirm a selected path, or quit.
enum EventAction {
    /// Continue running the event loop.
    Continue,
    /// Confirm the selection of a path, carrying the selected `PathBuf`.
    Confirm(PathBuf),
    /// Quit the application.
    Quit,
}

fn get_keymap() -> Keymap {
    match env::var("PD_KEYMAP").as_deref() {
        Ok("emacs") => Keymap::Emacs,
        Ok("vim") => Keymap::Vim,
        Ok(other) => {
            eprintln!(
                "Warning: Unknown PD_KEYMAP value '{}', defaulting to Vim",
                other
            );
            Keymap::Vim
        }
        Err(_) => Keymap::Vim,
    }
}

/// Splits a `Path` into string components suitable for display.
///
/// This function handles paths in a specific way to ensure each part includes its
/// separator, allowing them to be easily joined back into a valid path.
///
/// # Examples
/// - Unix: `/home/user/project` -> `["/", "home/", "user/", "project"]`
/// - Windows: `C:\Users\Admin` -> `["C:\", "Users\", "Admin"]`
fn split_path(path: &Path) -> Vec<String> {
    let mut parts = Vec::new();
    let mut components = path.components().peekable();

    while let Some(component) = components.next() {
        let is_last = components.peek().is_none();
        let part = match component {
            Component::Prefix(prefix) => {
                let mut p = prefix.as_os_str().to_string_lossy().to_string();
                // Special handling for Windows paths like "C:" followed by "\"
                if let Some(Component::RootDir) = components.peek() {
                    components.next(); // Consume the RootDir
                    p.push(std::path::MAIN_SEPARATOR);
                }
                p
            }
            Component::RootDir => std::path::MAIN_SEPARATOR_STR.to_string(),
            Component::Normal(s) => {
                let mut p = s.to_string_lossy().to_string();
                if !is_last {
                    p.push(std::path::MAIN_SEPARATOR);
                }
                p
            }
            // Ignore "." and ".." components
            _ => continue,
        };
        parts.push(part);
    }

    // Handle the edge case of an empty path or a "." path.
    if parts.is_empty() {
        parts.push(".".to_string());
    }

    parts
}

/// Renders the interactive path selection UI to the terminal.
///
/// This function clears the current line and then prints all path components.
/// The currently selected component is highlighted with a reverse attribute.
///
/// # Arguments
/// * `out`: A writable destination, typically `stderr`.
/// * `state`: The current state of the application.
fn render<W: Write>(out: &mut W, state: &AppState) -> Result<()> {
    execute!(
        out,
        cursor::MoveToColumn(0),
        // Clear(ClearType::FromCursorDown)
    )?;
    for (i, part) in state.path_parts.iter().enumerate() {
        if i == state.current_index {
            execute!(
                out,
                SetAttribute(Attribute::Reverse), // Set reverse video for selection
                Print(part),
                SetAttribute(Attribute::Reset) // Reset attributes
            )?;
        } else {
            execute!(out, Print(part))?;
        }
    }
    out.flush()
}

/// Processes Vim-style key bindings to navigate the path components.
///
/// This function updates the application state based on Vim key bindings such as
/// `h`, `j`, `k`, `l`, or arrow keys for navigation, and handles numeric prefixes
/// for repeated actions.
///
/// # Arguments
/// * `key`: The keyboard event to process.
/// * `state`: Mutable reference to the current application state.
fn handle_vim_keys(key: KeyEvent, state: &mut AppState) {
    match key.code {
        KeyCode::Char('h' | 'k' | 'b') | KeyCode::Left => state.move_by(-1),
        KeyCode::Char('l' | 'j' | 'w') | KeyCode::Right => state.move_by(1),
        KeyCode::Char('^' | 'H') | KeyCode::Home => state.move_to_start(),
        KeyCode::Char('$' | 'L') | KeyCode::End => state.move_to_end(),
        KeyCode::Char('M') => state.move_to_middle(),
        KeyCode::Char(c) if c.is_ascii_digit() => {
            // In Vim, '0' moves to the start unless it's part of a count.
            if c == '0' && state.count_input.is_empty() {
                state.move_to_start();
            } else {
                state.count_input.push(c);
            }
        }
        _ => {}
    }
}

/// Processes Emacs-style key bindings to navigate the path components.
///
/// This function updates the application state based on Emacs key bindings such as
/// `Ctrl-b`, `Ctrl-f`, `Alt-b`, `Alt-f`, or arrow keys for navigation.
///
/// # Arguments
/// * `key`: The keyboard event to process.
/// * `state`: Mutable reference to the current application state.
fn handle_emacs_keys(key: KeyEvent, state: &mut AppState) {
    const CTRL: KeyModifiers = KeyModifiers::CONTROL;
    const ALT: KeyModifiers = KeyModifiers::ALT;
    match key.code {
        // C-b, Alt-b, Left
        KeyCode::Char('b') if key.modifiers.contains(CTRL) => state.move_by(-1),
        KeyCode::Char('b') if key.modifiers.contains(ALT) => state.move_by(-1),
        KeyCode::Left => state.move_by(-1),
        // C-f, Alt-f, Right
        KeyCode::Char('f') if key.modifiers.contains(CTRL) => state.move_by(1),
        KeyCode::Char('f') if key.modifiers.contains(ALT) => state.move_by(1),
        KeyCode::Right => state.move_by(1),
        // C-a, Home
        KeyCode::Char('a') if key.modifiers.contains(CTRL) => state.move_to_start(),
        KeyCode::Home => state.move_to_start(),
        // C-e, End
        KeyCode::Char('e') if key.modifiers.contains(CTRL) => state.move_to_end(),
        KeyCode::End => state.move_to_end(),
        _ => {}
    }
}

/// [Unix-only] Handles the suspend signal (Ctrl+Z).
///
/// It first restores the terminal mode, then sends the `SIGTSTP` signal to itself
/// to suspend the process. When the process is resumed (e.g., with `fg`),
/// it re-enables raw mode.
#[cfg(unix)]
fn handle_suspend() -> Result<()> {
    let _ = restore_terminal_mode();
    signal::raise(Signal::SIGTSTP).expect("Failed to send SIGTSTP");
    // ... The process is suspended here ...
    // Code execution resumes here after the process is brought to the foreground.
    let _ = set_terminal_mode();
    Ok(())
}

/// [Unix-only] Handles the interrupt signal (Ctrl+C).
///
/// It restores the terminal and then re-raises the `SIGINT` signal to allow
/// the process to terminate gracefully as if it received the signal directly.
#[cfg(unix)]
fn handle_interrupt() {
    let _ = restore_terminal_mode();

    signal::raise(Signal::SIGINT).expect("Failed to send SIGINT");
    // `raise(SIGINT)` will terminate the process, so this is unreachable.
    unreachable!();
}

/// Handles keyboard events based on the selected keymap and updates the application state.
///
/// This function processes key presses, applying Vim or Emacs key bindings, and handles
/// special actions like confirming a selection or quitting.
///
/// # Arguments
/// * `key`: The keyboard event to process.
/// * `state`: Mutable reference to the current application state.
/// * `keymap`: The keymap mode (Vim or Emacs) to use for key bindings.
///
/// # Returns
/// * `Result<EventAction>`: Indicates the action to take (`Continue`, `Confirm`, or `Quit`).
fn handle_key_event(key: KeyEvent, state: &mut AppState, keymap: Keymap) -> Result<EventAction> {
    const CTRL: KeyModifiers = KeyModifiers::CONTROL;
    if let KeyEventKind::Press = key.kind {
        match keymap {
            Keymap::Vim => handle_vim_keys(key, state),
            Keymap::Emacs => handle_emacs_keys(key, state),
        }

        match key.code {
            // --- Shared Keys ---
            KeyCode::Enter => {
                return Ok(EventAction::Confirm(state.selected_path()));
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(EventAction::Quit);
            }
            // --- Signal Handling ---
            KeyCode::Char('c') if key.modifiers.contains(CTRL) => {
                // On Unix, emulate a true Ctrl+C interrupt.
                #[cfg(unix)]
                handle_interrupt();

                // On Windows, treat Ctrl+C as a "quit" action.
                #[cfg(not(unix))]
                return Ok(EventAction::Quit);
            }
            KeyCode::Char('z') if key.modifiers.contains(CTRL) => {
                // Ctrl+Z suspend is a Unix-only feature.
                #[cfg(unix)]
                let _ = handle_suspend();
            }
            _ => {}
        }
    }

    Ok(EventAction::Continue)
}

/// Handles mouse events and updates the application state.
///
/// This function processes mouse movements, clicks, and scroll events to navigate or select
/// path components.
///
/// # Arguments
/// * `mouse`: The mouse event to process.
/// * `state`: Mutable reference to the current application state.
///
/// # Returns
/// * `Result<EventAction>`: Indicates the action to take (`Continue`, `Confirm`, or `Quit`).
fn handle_mouse_event(mouse: MouseEvent, state: &mut AppState) -> Result<EventAction> {
    match mouse.kind {
        MouseEventKind::Moved => {
            state.select_part_at_column(mouse.column);
        }
        // FIXME: 'Up' event is unexpectedly reserved in MSYS2 after exit.
        MouseEventKind::Down(MouseButton::Left) => {
            return Ok(EventAction::Confirm(state.selected_path()));
        }
        MouseEventKind::Down(MouseButton::Right) => {
            return Ok(EventAction::Quit);
        }
        MouseEventKind::ScrollUp | MouseEventKind::ScrollLeft => {
            state.move_by(-1);
        }
        MouseEventKind::ScrollDown | MouseEventKind::ScrollRight => {
            state.move_by(1);
        }
        _ => {}
    }

    Ok(EventAction::Continue)
}

/// Handles events (keyboard or mouse) and updates the application state accordingly.
///
/// This function processes a single event, delegates to specific handlers based on the event type,
/// and returns an action indicating whether to continue, confirm a path, or quit.
///
/// # Arguments
/// * `event`: The input event (key press or mouse action) to process.
/// * `state`: Mutable reference to the current application state.
/// * `keymap`: The keymap mode (Vim or Emacs) to use for key bindings.
///
/// # Returns
/// * `Result<EventAction>`: Indicates the action to take (`Continue`, `Confirm`, or `Quit`).
fn handle_event(event: Event, state: &mut AppState, keymap: Keymap) -> Result<EventAction> {
    match event {
        Event::Key(key) => return handle_key_event(key, state, keymap),
        Event::Mouse(mouse) => return handle_mouse_event(mouse, state),
        _ => {}
    }

    Ok(EventAction::Continue)
}

/// Runs the main interactive event loop.
///
/// This function sets up the environment, listens for user input (keyboard and mouse),
/// updates the application state, and re-renders the UI.
///
/// # Returns
/// - `Ok(Some(PathBuf))`: If the user selects a path and presses Enter.
/// - `Ok(None)`: If the user quits with `q` or `Esc`.
/// - `Err(e)`: If an I/O error occurs during the process.
fn run_interactive_selector() -> Result<Option<PathBuf>> {
    let pwd = env::current_dir()?;

    let keymap = get_keymap();
    let path_parts = split_path(&pwd);
    let mut state = AppState::new(path_parts);
    // `_cleanup` ensures the terminal is restored when this function returns.
    let _cleanup = TermCleanup;
    let _ = set_terminal_mode();

    loop {
        render(&mut stderr(), &state)?;
        match handle_event(event::read()?, &mut state, keymap)? {
            EventAction::Continue => {}
            EventAction::Confirm(path) => return Ok(Some(path)),
            EventAction::Quit => return Ok(None),
        }
    }
}

/// The main entry point of the application.
///
/// This function calls the interactive selector and handles its result.
/// On success, it prints the chosen path to `stdout`. On quit or error,
/// it exits with a non-zero status code.
fn main() {
    match run_interactive_selector() {
        Ok(Some(path)) => {
            // The final path is printed to stdout for the shell to capture.
            println!("{}", path.to_string_lossy());
        }
        Ok(None) => {
            // User quit; exit with a non-zero status code.
            std::process::exit(1);
        }
        Err(e) => {
            // Ensure terminal is restored before printing the error.
            let _ = restore_terminal_mode();
            eprintln!("Error: {}", e);
            std::process::exit(2);
        }
    }
}
