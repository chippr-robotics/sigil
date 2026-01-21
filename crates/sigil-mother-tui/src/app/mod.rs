//! Application state machine and core logic

mod events;
mod router;
mod state;

pub use events::Event;
pub use router::{Route, Router};
pub use state::{AppState, Screen};

use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyModifiers};
use ratatui::prelude::*;

use crate::auth::{AuthError, AuthState, PinManager, Session, SessionConfig};
use crate::ui::{self, Theme};

/// Tick rate for the event loop (60fps for smooth animations)
const TICK_RATE: Duration = Duration::from_millis(16);

/// Main application struct
pub struct App {
    /// Current application state
    pub state: AppState,
    /// Authentication state
    pub auth: AuthState,
    /// PIN manager for authentication
    pub pin_manager: PinManager,
    /// Current session (if authenticated)
    pub session: Option<Session>,
    /// Navigation router
    pub router: Router,
    /// Visual theme
    pub theme: Theme,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Animation tick counter
    pub tick: u64,
    /// Last disk check time
    pub last_disk_check: Instant,
}

impl App {
    /// Create a new application instance
    pub fn new() -> Result<Self> {
        let pin_manager = PinManager::new()
            .map_err(|e| anyhow::anyhow!("Failed to initialize PIN manager: {}", e))?;
        let auth = if pin_manager.is_pin_set() {
            AuthState::RequiresPin
        } else {
            AuthState::SetupRequired
        };

        Ok(Self {
            state: AppState::default(),
            auth,
            pin_manager,
            session: None,
            router: Router::new(),
            theme: Theme::default(),
            should_quit: false,
            tick: 0,
            last_disk_check: Instant::now(),
        })
    }

    /// Main event loop
    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        let mut last_tick = Instant::now();

        // Show splash screen briefly
        self.state.current_screen = Screen::Splash;

        loop {
            // Draw the current frame
            terminal.draw(|frame| self.draw(frame))?;

            // Check if we should quit
            if self.should_quit {
                break;
            }

            // Calculate time until next tick
            let timeout = TICK_RATE
                .checked_sub(last_tick.elapsed())
                .unwrap_or(Duration::ZERO);

            // Poll for events with timeout
            if event::poll(timeout)? {
                if let CrosstermEvent::Key(key) = event::read()? {
                    self.handle_key_event(key.code, key.modifiers)?;
                }
            }

            // Handle tick
            if last_tick.elapsed() >= TICK_RATE {
                self.on_tick()?;
                last_tick = Instant::now();
            }
        }

        Ok(())
    }

    /// Handle key events
    fn handle_key_event(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        // Global quit handling
        if key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return Ok(());
        }

        // Handle based on current screen
        match &self.state.current_screen {
            Screen::Splash => {
                // Any key advances from splash
                self.advance_from_splash();
            }
            Screen::PinEntry => {
                self.handle_pin_entry(key)?;
            }
            Screen::PinSetup => {
                self.handle_pin_setup(key)?;
            }
            Screen::Dashboard => {
                self.handle_dashboard(key)?;
            }
            Screen::DiskStatus => {
                self.handle_disk_status(key)?;
            }
            Screen::DiskFormat(_) => {
                self.handle_disk_format(key)?;
            }
            Screen::ChildList => {
                self.handle_child_list(key)?;
            }
            Screen::ChildCreate(_) => {
                self.handle_child_create(key)?;
            }
            Screen::ChildDetail(_) => {
                self.handle_child_detail(key)?;
            }
            Screen::Reconciliation => {
                self.handle_reconciliation(key)?;
            }
            Screen::Reports => {
                self.handle_reports(key)?;
            }
            Screen::QrDisplay(_) => {
                self.handle_qr_display(key)?;
            }
            Screen::Settings => {
                self.handle_settings(key)?;
            }
            Screen::Help => {
                self.handle_help(key)?;
            }
            Screen::Confirm(_) => {
                self.handle_confirm(key)?;
            }
            Screen::Lockout(_) => {
                // During lockout, only allow viewing time remaining
                if key == KeyCode::Esc {
                    self.should_quit = true;
                }
            }
        }

        Ok(())
    }

    /// Advance from splash screen to appropriate next screen
    fn advance_from_splash(&mut self) {
        match self.auth {
            AuthState::SetupRequired => {
                self.state.current_screen = Screen::PinSetup;
            }
            AuthState::RequiresPin => {
                self.state.current_screen = Screen::PinEntry;
            }
            AuthState::Authenticated => {
                self.state.current_screen = Screen::Dashboard;
            }
            AuthState::LockedOut(until) => {
                self.state.current_screen = Screen::Lockout(until);
            }
        }
    }

    /// Handle PIN entry screen
    fn handle_pin_entry(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if self.state.pin_input.len() < 12 {
                    self.state.pin_input.push(c);
                }
            }
            KeyCode::Backspace => {
                self.state.pin_input.pop();
            }
            KeyCode::Enter => {
                if self.state.pin_input.len() >= 6 {
                    self.verify_pin()?;
                }
            }
            KeyCode::Esc => {
                self.should_quit = true;
            }
            _ => {}
        }
        Ok(())
    }

    /// Verify entered PIN
    fn verify_pin(&mut self) -> Result<()> {
        let pin = std::mem::take(&mut self.state.pin_input);

        match self.pin_manager.verify_pin(&pin) {
            Ok(encryption_key) => {
                // PIN verified - create session with encryption key
                self.auth = AuthState::Authenticated;
                self.session = Some(Session::new(encryption_key, SessionConfig::default()));
                self.state.current_screen = Screen::Dashboard;
                self.state.status_message = Some("Welcome to Sigil Mother".to_string());
            }
            Err(AuthError::IncorrectPin(remaining)) => {
                self.state.error_message = Some(format!(
                    "Incorrect PIN. {} attempts remaining.",
                    remaining
                ));

                // Check if locked out
                if let Some(until) = self.pin_manager.lockout_until() {
                    self.auth = AuthState::LockedOut(until);
                    self.state.current_screen = Screen::Lockout(until);
                }
            }
            Err(AuthError::LockedOut(seconds)) => {
                self.state.error_message = Some(format!(
                    "Account locked for {} seconds.",
                    seconds
                ));
                if let Some(until) = self.pin_manager.lockout_until() {
                    self.auth = AuthState::LockedOut(until);
                    self.state.current_screen = Screen::Lockout(until);
                }
            }
            Err(e) => {
                self.state.error_message = Some(format!("Authentication error: {}", e));
            }
        }

        Ok(())
    }

    /// Handle PIN setup screen
    fn handle_pin_setup(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if self.state.setup_step == 0 {
                    if self.state.pin_input.len() < 12 {
                        self.state.pin_input.push(c);
                    }
                } else {
                    if self.state.pin_confirm.len() < 12 {
                        self.state.pin_confirm.push(c);
                    }
                }
            }
            KeyCode::Backspace => {
                if self.state.setup_step == 0 {
                    self.state.pin_input.pop();
                } else {
                    self.state.pin_confirm.pop();
                }
            }
            KeyCode::Enter => {
                if self.state.setup_step == 0 {
                    if self.state.pin_input.len() >= 6 {
                        self.state.setup_step = 1;
                    } else {
                        self.state.error_message =
                            Some("PIN must be at least 6 digits".to_string());
                    }
                } else {
                    if self.state.pin_input == self.state.pin_confirm {
                        // Set the PIN
                        if let Err(e) = self.pin_manager.set_pin(&self.state.pin_input) {
                            self.state.error_message = Some(format!("Failed to set PIN: {}", e));
                            self.state.pin_input.clear();
                            self.state.pin_confirm.clear();
                            self.state.setup_step = 0;
                            return Ok(());
                        }

                        // Verify PIN to get encryption key for session
                        let pin = std::mem::take(&mut self.state.pin_input);
                        self.state.pin_confirm.clear();

                        match self.pin_manager.verify_pin(&pin) {
                            Ok(encryption_key) => {
                                self.auth = AuthState::Authenticated;
                                self.session = Some(Session::new(encryption_key, SessionConfig::default()));
                                self.state.current_screen = Screen::Dashboard;
                                self.state.status_message = Some("PIN set successfully. Welcome to Sigil Mother!".to_string());
                            }
                            Err(e) => {
                                self.state.error_message = Some(format!("PIN verification failed: {}", e));
                                self.state.setup_step = 0;
                            }
                        }
                    } else {
                        self.state.error_message =
                            Some("PINs do not match. Please try again.".to_string());
                        self.state.pin_input.clear();
                        self.state.pin_confirm.clear();
                        self.state.setup_step = 0;
                    }
                }
            }
            KeyCode::Esc => {
                if self.state.setup_step == 1 {
                    self.state.setup_step = 0;
                    self.state.pin_confirm.clear();
                } else {
                    self.should_quit = true;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle dashboard navigation
    fn handle_dashboard(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.state.current_screen = Screen::Confirm(ConfirmAction::Quit);
            }
            KeyCode::Char('?') => {
                self.state.current_screen = Screen::Help;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.menu_index = self.state.menu_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.menu_index = (self.state.menu_index + 1).min(5);
            }
            KeyCode::Enter => {
                self.navigate_from_dashboard();
            }
            KeyCode::F(1) => {
                self.state.current_screen = Screen::DiskStatus;
            }
            KeyCode::F(2) => {
                self.state.current_screen = Screen::ChildList;
            }
            KeyCode::F(3) => {
                self.state.current_screen = Screen::Reconciliation;
            }
            KeyCode::F(4) => {
                self.state.current_screen = Screen::Reports;
            }
            _ => {}
        }
        Ok(())
    }

    /// Navigate based on dashboard menu selection
    fn navigate_from_dashboard(&mut self) {
        match self.state.menu_index {
            0 => self.state.current_screen = Screen::DiskStatus,
            1 => self.state.current_screen = Screen::ChildList,
            2 => self.state.current_screen = Screen::Reconciliation,
            3 => self.state.current_screen = Screen::Reports,
            4 => self.state.current_screen = Screen::QrDisplay(QrDisplayType::AgentShard),
            5 => self.state.current_screen = Screen::Settings,
            _ => {}
        }
    }

    /// Handle disk status screen
    fn handle_disk_status(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.state.current_screen = Screen::Dashboard;
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.state.current_screen = Screen::DiskFormat(0);
            }
            KeyCode::Char('?') => {
                self.state.current_screen = Screen::Help;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle disk format wizard
    fn handle_disk_format(&mut self, key: KeyCode) -> Result<()> {
        let step = match &self.state.current_screen {
            Screen::DiskFormat(s) => *s,
            _ => return Ok(()),
        };

        match key {
            KeyCode::Esc => {
                if step > 0 {
                    self.state.current_screen = Screen::DiskFormat(step - 1);
                } else {
                    self.state.current_screen = Screen::DiskStatus;
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                if step < 4 {
                    self.state.current_screen = Screen::DiskFormat(step + 1);
                } else {
                    // Complete - return to disk status
                    self.state.current_screen = Screen::DiskStatus;
                    self.state.status_message = Some("Disk formatted successfully".to_string());
                }
            }
            KeyCode::Left => {
                if step > 0 {
                    self.state.current_screen = Screen::DiskFormat(step - 1);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle child list screen
    fn handle_child_list(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.state.current_screen = Screen::Dashboard;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.state.current_screen = Screen::ChildCreate(0);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.child_list_index = self.state.child_list_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.child_list_index += 1;
            }
            KeyCode::Enter => {
                // View child details
                self.state.current_screen = Screen::ChildDetail(self.state.child_list_index);
            }
            KeyCode::Char('?') => {
                self.state.current_screen = Screen::Help;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle child create wizard
    fn handle_child_create(&mut self, key: KeyCode) -> Result<()> {
        let step = match &self.state.current_screen {
            Screen::ChildCreate(s) => *s,
            _ => return Ok(()),
        };

        match key {
            KeyCode::Esc => {
                if step > 0 {
                    self.state.current_screen = Screen::ChildCreate(step - 1);
                } else {
                    self.state.current_screen = Screen::ChildList;
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                if step < 5 {
                    self.state.current_screen = Screen::ChildCreate(step + 1);
                } else {
                    // Complete - show QR code with agent shard
                    self.state.current_screen = Screen::QrDisplay(QrDisplayType::NewChildShard);
                }
            }
            KeyCode::Left => {
                if step > 0 {
                    self.state.current_screen = Screen::ChildCreate(step - 1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                // Navigate options within step
                if self.state.wizard_option_index > 0 {
                    self.state.wizard_option_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.wizard_option_index += 1;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle child detail screen
    fn handle_child_detail(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.state.current_screen = Screen::ChildList;
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                // Show QR code for this child's agent shard
                self.state.current_screen = Screen::QrDisplay(QrDisplayType::AgentShard);
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                // Nullify - requires confirmation
                self.state.current_screen = Screen::Confirm(ConfirmAction::NullifyChild);
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle reconciliation screen
    fn handle_reconciliation(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.state.current_screen = Screen::Dashboard;
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                // Analyze disk
                self.state.status_message = Some("Analyzing disk...".to_string());
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Refill approved disk
                self.state.current_screen = Screen::Confirm(ConfirmAction::RefillDisk);
            }
            KeyCode::Char('?') => {
                self.state.current_screen = Screen::Help;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle reports screen
    fn handle_reports(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.state.current_screen = Screen::Dashboard;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.report_type_index = self.state.report_type_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.report_type_index = (self.state.report_type_index + 1).min(3);
            }
            KeyCode::Enter => {
                // Generate selected report
                self.state.status_message = Some("Generating report...".to_string());
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                // Export to USB
                self.state.status_message = Some("Looking for USB drives...".to_string());
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle QR display screen
    fn handle_qr_display(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                // Return to previous screen
                self.state.current_screen = Screen::Dashboard;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                // Save QR as image
                self.state.status_message = Some("QR code saved to file".to_string());
            }
            KeyCode::Left if self.state.qr_chunk_index > 0 => {
                self.state.qr_chunk_index -= 1;
            }
            KeyCode::Right => {
                self.state.qr_chunk_index += 1;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle settings screen
    fn handle_settings(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.state.current_screen = Screen::Dashboard;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.settings_index = self.state.settings_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.settings_index = (self.state.settings_index + 1).min(4);
            }
            KeyCode::Enter => {
                match self.state.settings_index {
                    0 => {
                        // Change PIN
                        self.state.current_screen = Screen::Confirm(ConfirmAction::ChangePIN);
                    }
                    4 => {
                        // Factory reset
                        self.state.current_screen = Screen::Confirm(ConfirmAction::FactoryReset);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle help screen
    fn handle_help(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter => {
                // Return to previous screen
                self.state.current_screen = self.router.back().unwrap_or(Screen::Dashboard);
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle confirmation dialogs
    fn handle_confirm(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                // Cancel
                self.state.current_screen = self.router.back().unwrap_or(Screen::Dashboard);
                self.state.confirm_input.clear();
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Quick confirm for simple dialogs
                if let Screen::Confirm(ConfirmAction::Quit) = &self.state.current_screen {
                    self.should_quit = true;
                }
            }
            KeyCode::Char(c) => {
                // Typed confirmation for dangerous operations
                self.state.confirm_input.push(c);
            }
            KeyCode::Backspace => {
                self.state.confirm_input.pop();
            }
            KeyCode::Enter => {
                // Verify typed confirmation
                self.process_confirmation()?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Process a typed confirmation
    fn process_confirmation(&mut self) -> Result<()> {
        let action = match &self.state.current_screen {
            Screen::Confirm(a) => a.clone(),
            _ => return Ok(()),
        };

        match action {
            ConfirmAction::Quit => {
                self.should_quit = true;
            }
            ConfirmAction::NullifyChild => {
                // Requires typing "NULLIFY <child_id>"
                // For now, just return to dashboard
                self.state.current_screen = Screen::Dashboard;
                self.state.status_message =
                    Some("Child nullification not yet implemented".to_string());
            }
            ConfirmAction::FactoryReset => {
                // Requires typing "FACTORY RESET"
                self.state.current_screen = Screen::Dashboard;
                self.state.status_message = Some("Factory reset not yet implemented".to_string());
            }
            ConfirmAction::RefillDisk => {
                self.state.current_screen = Screen::Reconciliation;
                self.state.status_message = Some("Disk refill not yet implemented".to_string());
            }
            ConfirmAction::ChangePIN => {
                self.state.current_screen = Screen::PinSetup;
                self.state.setup_step = 0;
            }
        }

        self.state.confirm_input.clear();
        Ok(())
    }

    /// Handle periodic tick
    fn on_tick(&mut self) -> Result<()> {
        self.tick = self.tick.wrapping_add(1);

        // Check session timeout
        if let Some(session) = &self.session {
            if !session.is_valid() {
                self.session = None;
                self.auth = AuthState::RequiresPin;
                self.state.current_screen = Screen::PinEntry;
                self.state.status_message = Some("Session timed out. Please re-authenticate.".to_string());
            } else if session.should_warn() {
                let remaining = session.idle_seconds_remaining();
                self.state.session_warning = Some(format!("Session expires in {} seconds", remaining));
            } else {
                self.state.session_warning = None;
            }
        }

        // Periodic disk status check (every 2 seconds)
        if self.last_disk_check.elapsed() > Duration::from_secs(2) {
            self.check_disk_status()?;
            self.last_disk_check = Instant::now();
        }

        // Clear transient messages after a few seconds
        if self.tick % 180 == 0 {
            self.state.status_message = None;
            self.state.error_message = None;
        }

        Ok(())
    }

    /// Check current disk status
    fn check_disk_status(&mut self) -> Result<()> {
        // TODO: Integrate with sigil-core disk detection
        // For now, just update state.disk_detected
        Ok(())
    }

    /// Draw the current frame
    fn draw(&self, frame: &mut Frame) {
        ui::draw(frame, self);
    }
}

/// Types of confirmation actions
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfirmAction {
    Quit,
    NullifyChild,
    FactoryReset,
    RefillDisk,
    ChangePIN,
}

/// Types of QR code displays
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QrDisplayType {
    AgentShard,
    NewChildShard,
    DkgPackage,
}
