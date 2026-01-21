//! Application state and event handling

mod state;

pub use state::{AppState, Screen};

use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;

use crate::ui;

/// Application result type
pub type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Main application struct
pub struct App {
    /// Application state
    pub state: AppState,

    /// Whether the app should quit
    pub should_quit: bool,

    /// Tick counter for animations
    pub tick: u64,

    /// Last tick time
    last_tick: Instant,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Create a new application instance
    pub fn new() -> Self {
        Self {
            state: AppState::new(),
            should_quit: false,
            tick: 0,
            last_tick: Instant::now(),
        }
    }

    /// Run the application main loop
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> AppResult<()> {
        let tick_rate = Duration::from_millis(100);

        while !self.should_quit {
            // Draw UI
            terminal.draw(|frame| ui::render(frame, &mut self.state))?;

            // Handle events
            let timeout = tick_rate
                .checked_sub(self.last_tick.elapsed())
                .unwrap_or(Duration::ZERO);

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code);
                    }
                }
            }

            // Update tick
            if self.last_tick.elapsed() >= tick_rate {
                self.tick = self.tick.wrapping_add(1);
                self.last_tick = Instant::now();
            }
        }

        Ok(())
    }

    /// Handle key press events
    fn handle_key(&mut self, key: KeyCode) {
        // Global quit handler
        if key == KeyCode::Char('q') && self.state.current_screen == Screen::Dashboard {
            self.should_quit = true;
            return;
        }

        // Delegate to screen-specific handlers
        match self.state.current_screen {
            Screen::Splash => self.handle_splash_key(key),
            Screen::Dashboard => self.handle_dashboard_key(key),
            Screen::AgentList => self.handle_agent_list_key(key),
            Screen::AgentDetail => self.handle_agent_detail_key(key),
            Screen::AgentCreate => self.handle_agent_create_key(key),
            Screen::AgentNullify => self.handle_agent_nullify_key(key),
            Screen::ChildList => self.handle_child_list_key(key),
            Screen::ChildCreate => self.handle_child_create_key(key),
            Screen::QrDisplay => self.handle_qr_display_key(key),
            Screen::Help => self.handle_help_key(key),
        }
    }

    fn handle_splash_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.state.current_screen = Screen::Dashboard;
            }
            _ => {}
        }
    }

    fn handle_dashboard_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.menu_index > 0 {
                    self.state.menu_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.state.menu_index < 5 {
                    self.state.menu_index += 1;
                }
            }
            KeyCode::Enter => {
                match self.state.menu_index {
                    0 => self.state.current_screen = Screen::ChildList, // Children
                    1 => self.state.current_screen = Screen::AgentList, // Agents
                    2 => {} // Reconciliation (not implemented)
                    3 => {} // Reports (not implemented)
                    4 => self.state.current_screen = Screen::Help, // Help
                    5 => self.should_quit = true, // Quit
                    _ => {}
                }
            }
            KeyCode::Char('?') => {
                self.state.current_screen = Screen::Help;
            }
            _ => {}
        }
    }

    fn handle_agent_list_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('b') => {
                self.state.current_screen = Screen::Dashboard;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.agent_list_index > 0 {
                    self.state.agent_list_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let agent_count = self.state.agent_registry.list_all().len();
                if self.state.agent_list_index < agent_count.saturating_sub(1) {
                    self.state.agent_list_index += 1;
                }
            }
            KeyCode::Enter => {
                if !self.state.agent_registry.list_all().is_empty() {
                    self.state.current_screen = Screen::AgentDetail;
                }
            }
            KeyCode::Char('n') => {
                self.state.current_screen = Screen::AgentCreate;
                self.state.agent_create_step = 0;
                self.state.agent_name_input.clear();
            }
            _ => {}
        }
    }

    fn handle_agent_detail_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('b') => {
                self.state.current_screen = Screen::AgentList;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.agent_action_index > 0 {
                    self.state.agent_action_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.state.agent_action_index < 3 {
                    self.state.agent_action_index += 1;
                }
            }
            KeyCode::Enter => {
                match self.state.agent_action_index {
                    0 => {} // View witness (not implemented)
                    1 => {} // Suspend/Reactivate
                    2 => {
                        self.state.current_screen = Screen::AgentNullify;
                        self.state.nullify_confirmed = false;
                    }
                    3 => self.state.current_screen = Screen::AgentList, // Back
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn handle_agent_create_key(&mut self, key: KeyCode) {
        match self.state.agent_create_step {
            0 => {
                // Name input step
                match key {
                    KeyCode::Esc => {
                        self.state.current_screen = Screen::AgentList;
                    }
                    KeyCode::Enter => {
                        if !self.state.agent_name_input.is_empty() {
                            self.state.agent_create_step = 1;
                        }
                    }
                    KeyCode::Backspace => {
                        self.state.agent_name_input.pop();
                    }
                    KeyCode::Char(c) => {
                        if self.state.agent_name_input.len() < 32 {
                            self.state.agent_name_input.push(c);
                        }
                    }
                    _ => {}
                }
            }
            1 => {
                // Confirm step
                match key {
                    KeyCode::Esc => {
                        self.state.agent_create_step = 0;
                    }
                    KeyCode::Enter | KeyCode::Char('y') => {
                        // Create the agent
                        self.create_agent();
                        self.state.current_screen = Screen::AgentList;
                    }
                    KeyCode::Char('n') => {
                        self.state.agent_create_step = 0;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn handle_agent_nullify_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('n') => {
                self.state.current_screen = Screen::AgentDetail;
            }
            KeyCode::Char('y') if !self.state.nullify_confirmed => {
                self.state.nullify_confirmed = true;
            }
            KeyCode::Enter if self.state.nullify_confirmed => {
                self.nullify_selected_agent();
                self.state.current_screen = Screen::AgentList;
            }
            _ => {}
        }
    }

    fn handle_child_list_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('b') => {
                self.state.current_screen = Screen::Dashboard;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.child_list_index > 0 {
                    self.state.child_list_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let child_count = self.state.child_registry.list_all().len();
                if self.state.child_list_index < child_count.saturating_sub(1) {
                    self.state.child_list_index += 1;
                }
            }
            KeyCode::Char('n') => {
                self.state.current_screen = Screen::ChildCreate;
            }
            _ => {}
        }
    }

    fn handle_child_create_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.state.current_screen = Screen::ChildList;
            }
            _ => {}
        }
    }

    fn handle_qr_display_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Enter => {
                self.state.current_screen = Screen::Dashboard;
            }
            KeyCode::Left => {
                if self.state.qr_chunk_index > 0 {
                    self.state.qr_chunk_index -= 1;
                }
            }
            KeyCode::Right => {
                if self.state.qr_chunk_index < self.state.qr_total_chunks.saturating_sub(1) {
                    self.state.qr_chunk_index += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_help_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                self.state.current_screen = Screen::Dashboard;
            }
            _ => {}
        }
    }

    /// Create a new agent from current input
    fn create_agent(&mut self) {
        use sigil_core::agent::AgentId;

        // Generate a random agent ID (in real usage, this would come from agent's public key)
        let mut id_bytes = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut id_bytes);
        let agent_id = AgentId::new(id_bytes);

        let _ = self
            .state
            .agent_registry
            .register_agent(agent_id, self.state.agent_name_input.clone());

        self.state.agent_name_input.clear();
        self.state.agent_create_step = 0;
        self.state.status_message = Some("Agent created successfully".to_string());
    }

    /// Nullify the currently selected agent
    fn nullify_selected_agent(&mut self) {
        let agents = self.state.agent_registry.list_all();
        if let Some(entry) = agents.get(self.state.agent_list_index) {
            let agent_id = entry.agent_id;
            if self.state.agent_registry.nullify_agent(&agent_id).is_ok() {
                self.state.status_message = Some("Agent nullified".to_string());
            }
        }
        self.state.nullify_confirmed = false;
    }
}
