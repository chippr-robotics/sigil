//! Application state

use sigil_mother::{AgentRegistry, BlockDevice, ChildRegistry, DiskStatus, FloppyManager, MountMethod};

use super::config::TuiConfig;

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

    /// Disk management screen
    DiskManagement,

    /// Disk device selection screen
    DiskSelect,

    /// Disk format confirmation
    DiskFormat,

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

    /// Floppy disk manager
    pub floppy_manager: FloppyManager,

    /// Current disk status
    pub disk_status: Option<DiskStatus>,

    /// Disk action menu index
    pub disk_action_index: usize,

    /// Format type selection index (0 = ext2, 1 = FAT12)
    pub format_type_index: usize,

    /// Whether format is confirmed
    pub format_confirmed: bool,

    /// Available removable block devices
    pub available_devices: Vec<BlockDevice>,

    /// Device selection index in the device list
    pub device_select_index: usize,

    /// Currently selected device path (persisted)
    pub selected_device_path: Option<String>,

    /// TUI configuration (persisted)
    pub config: TuiConfig,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Create new application state
    pub fn new() -> Self {
        // Load persisted configuration
        let config = TuiConfig::load();

        // Create floppy manager with config settings
        let mut floppy_manager = FloppyManager::new()
            .with_mount_method(MountMethod::from(config.mount_method));

        // Apply selected device from config if set
        if let Some(ref device) = config.selected_device {
            floppy_manager.set_device(device);
        }

        // Set mount point from config
        floppy_manager.set_mount_point(&config.mount_point);

        let disk_status = Some(floppy_manager.check_status());

        // Try to load available devices at startup
        let available_devices = sigil_mother::list_removable_devices().unwrap_or_default();

        // Restore selected device path from config
        let selected_device_path = config.selected_device.clone();

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
            floppy_manager,
            disk_status,
            disk_action_index: 0,
            format_type_index: 0,
            format_confirmed: false,
            available_devices,
            device_select_index: 0,
            selected_device_path,
            config,
        }
    }

    /// Refresh disk status
    pub fn refresh_disk_status(&mut self) {
        self.disk_status = Some(self.floppy_manager.check_status());
    }

    /// Refresh available devices list
    pub fn refresh_available_devices(&mut self) {
        match sigil_mother::list_removable_devices() {
            Ok(devices) => {
                self.available_devices = devices;
                // Reset index if it's out of bounds
                if self.device_select_index >= self.available_devices.len() {
                    self.device_select_index = 0;
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to list devices: {}", e));
            }
        }
    }

    /// Select a device by index and update the floppy manager
    pub fn select_device(&mut self, index: usize) {
        if let Some(device) = self.available_devices.get(index) {
            self.selected_device_path = Some(device.path.clone());
            self.floppy_manager.set_device(&device.path);
            self.status_message = Some(format!("Selected device: {}", device.display_name()));

            // Persist the selection
            if let Err(e) = self.config.set_selected_device(Some(device.path.clone())) {
                tracing::warn!("Failed to save config: {}", e);
            }

            self.refresh_disk_status();
        }
    }

    /// Get the currently selected device info
    pub fn selected_device(&self) -> Option<&BlockDevice> {
        self.selected_device_path.as_ref().and_then(|path| {
            self.available_devices.iter().find(|d| &d.path == path)
        })
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
