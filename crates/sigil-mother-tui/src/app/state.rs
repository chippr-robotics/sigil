//! Application state structures

use std::time::Instant;

use super::{ConfirmAction, QrDisplayType};

/// Current screen/view in the application
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Screen {
    /// Initial splash screen with animated logo
    #[default]
    Splash,
    /// PIN entry for authentication
    PinEntry,
    /// First-time PIN setup
    PinSetup,
    /// Account lockout screen
    Lockout(Instant),
    /// Main dashboard
    Dashboard,
    /// Disk status view
    DiskStatus,
    /// Disk format wizard (step number)
    DiskFormat(u8),
    /// List of all children
    ChildList,
    /// Create child wizard (step number)
    ChildCreate(u8),
    /// Child detail view (index)
    ChildDetail(usize),
    /// Reconciliation workflow
    Reconciliation,
    /// Reports screen
    Reports,
    /// QR code display
    QrDisplay(QrDisplayType),
    /// Settings screen
    Settings,
    /// Help screen
    Help,
    /// Confirmation dialog
    Confirm(ConfirmAction),
}

/// Main application state
#[derive(Default)]
pub struct AppState {
    /// Current screen being displayed
    pub current_screen: Screen,

    // --- PIN Entry State ---
    /// Current PIN input (masked)
    pub pin_input: String,
    /// PIN confirmation for setup
    pub pin_confirm: String,
    /// Setup step (0 = enter, 1 = confirm)
    pub setup_step: u8,

    // --- Navigation State ---
    /// Selected menu item on dashboard
    pub menu_index: usize,
    /// Selected child in list
    pub child_list_index: usize,
    /// Selected report type
    pub report_type_index: usize,
    /// Selected settings item
    pub settings_index: usize,
    /// Current wizard option selection
    pub wizard_option_index: usize,

    // --- QR Display State ---
    /// Current QR chunk index (for multi-chunk QRs)
    pub qr_chunk_index: usize,
    /// Total QR chunks
    pub qr_total_chunks: usize,

    // --- Disk State ---
    /// Whether a disk is currently detected
    pub disk_detected: bool,
    /// Current disk child ID (if detected)
    pub disk_child_id: Option<String>,
    /// Presignatures remaining
    pub disk_presigs_remaining: Option<u32>,
    /// Total presignatures
    pub disk_presigs_total: Option<u32>,
    /// Days until expiry
    pub disk_days_until_expiry: Option<i64>,

    // --- Confirmation State ---
    /// Input for typed confirmation
    pub confirm_input: String,

    // --- Status Messages ---
    /// Current status message to display
    pub status_message: Option<String>,
    /// Current error message to display
    pub error_message: Option<String>,
    /// Session timeout warning
    pub session_warning: Option<String>,

    // --- Child Creation Wizard ---
    /// Selected signature scheme (0=ECDSA, 1=Taproot, 2=Ed25519)
    pub selected_scheme: usize,
    /// Number of presignatures to generate
    pub presig_count: u32,
    /// Validity period in days
    pub validity_days: u32,
}

impl AppState {
    /// Create a new state with defaults
    pub fn new() -> Self {
        Self {
            presig_count: 1000,
            validity_days: 30,
            ..Default::default()
        }
    }

    /// Get masked PIN display (dots)
    pub fn masked_pin(&self) -> String {
        "●".repeat(self.pin_input.len())
    }

    /// Get masked confirm PIN display
    pub fn masked_confirm(&self) -> String {
        "●".repeat(self.pin_confirm.len())
    }

    /// Clear all PIN-related input
    pub fn clear_pin_input(&mut self) {
        self.pin_input.clear();
        self.pin_confirm.clear();
        self.setup_step = 0;
    }
}
