//! Application state

use sigil_mother::{AgentRegistry, ChildRegistry};

/// Current screen/view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    /// Splash/welcome screen
    #[default]
    Splash,

    /// Main dashboard with menu
    Dashboard,

    /// List of registered agents
    AgentList,

    /// Agent detail view
    AgentDetail,

    /// Create new agent
    AgentCreate,

    /// Nullify agent confirmation
    AgentNullify,

    /// List of child disks
    ChildList,

    /// Create new child disk
    ChildCreate,

    /// QR code display
    QrDisplay,

    /// Help screen
    Help,
}

/// Application state
pub struct AppState {
    /// Current screen
    pub current_screen: Screen,

    /// Dashboard menu selection index
    pub menu_index: usize,

    /// Agent list selection index
    pub agent_list_index: usize,

    /// Agent detail action index
    pub agent_action_index: usize,

    /// Child list selection index
    pub child_list_index: usize,

    /// Agent creation step
    pub agent_create_step: u8,

    /// Agent name input buffer
    pub agent_name_input: String,

    /// Whether nullification is confirmed
    pub nullify_confirmed: bool,

    /// QR display: current chunk index
    pub qr_chunk_index: usize,

    /// QR display: total chunks
    pub qr_total_chunks: usize,

    /// QR display: data to show
    pub qr_data: Option<String>,

    /// Agent registry
    pub agent_registry: AgentRegistry,

    /// Child registry
    pub child_registry: ChildRegistry,

    /// Status message to display
    pub status_message: Option<String>,

    /// Error message to display
    pub error_message: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Create new application state
    pub fn new() -> Self {
        Self {
            current_screen: Screen::Splash,
            menu_index: 0,
            agent_list_index: 0,
            agent_action_index: 0,
            child_list_index: 0,
            agent_create_step: 0,
            agent_name_input: String::new(),
            nullify_confirmed: false,
            qr_chunk_index: 0,
            qr_total_chunks: 1,
            qr_data: None,
            agent_registry: AgentRegistry::new(),
            child_registry: ChildRegistry::new(),
            status_message: None,
            error_message: None,
        }
    }

    /// Get currently selected agent (if any)
    pub fn selected_agent(&self) -> Option<&sigil_core::agent::AgentRegistryEntry> {
        let agents = self.agent_registry.list_all();
        agents.get(self.agent_list_index).copied()
    }

    /// Get currently selected child (if any)
    pub fn selected_child(&self) -> Option<&sigil_core::child::ChildRegistryEntry> {
        let children = self.child_registry.list_all();
        children.get(self.child_list_index).copied()
    }

    /// Clear status messages
    pub fn clear_messages(&mut self) {
        self.status_message = None;
        self.error_message = None;
    }
}
