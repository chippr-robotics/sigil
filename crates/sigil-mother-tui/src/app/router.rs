//! Navigation router for screen transitions

use super::state::Screen;

/// Route represents a navigation path
#[derive(Clone, Debug)]
pub struct Route {
    /// The screen to display
    pub screen: Screen,
    /// Title for breadcrumb
    pub title: String,
}

impl Route {
    /// Create a new route
    pub fn new(screen: Screen, title: impl Into<String>) -> Self {
        Self {
            screen,
            title: title.into(),
        }
    }
}

/// Router manages navigation history
pub struct Router {
    /// Navigation history stack
    history: Vec<Route>,
    /// Maximum history depth
    max_depth: usize,
}

impl Router {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            max_depth: 20,
        }
    }

    /// Push a new route onto the history
    pub fn push(&mut self, screen: Screen, title: impl Into<String>) {
        let route = Route::new(screen, title);

        // Limit history depth
        if self.history.len() >= self.max_depth {
            self.history.remove(0);
        }

        self.history.push(route);
    }

    /// Go back to the previous screen
    pub fn back(&mut self) -> Option<Screen> {
        self.history.pop();
        self.history.last().map(|r| r.screen.clone())
    }

    /// Get the current route
    pub fn current(&self) -> Option<&Route> {
        self.history.last()
    }

    /// Get the breadcrumb trail
    pub fn breadcrumb(&self) -> Vec<&str> {
        self.history.iter().map(|r| r.title.as_str()).collect()
    }

    /// Clear navigation history
    pub fn clear(&mut self) {
        self.history.clear();
    }

    /// Check if we can go back
    pub fn can_go_back(&self) -> bool {
        self.history.len() > 1
    }

    /// Get title for a screen
    pub fn screen_title(screen: &Screen) -> &'static str {
        match screen {
            Screen::Splash => "Sigil Mother",
            Screen::PinEntry => "Authentication",
            Screen::PinSetup => "PIN Setup",
            Screen::Lockout(_) => "Locked",
            Screen::Dashboard => "Dashboard",
            Screen::DiskStatus => "Disk Status",
            Screen::DiskFormat(_) => "Format Disk",
            Screen::ChildList => "Children",
            Screen::ChildCreate(_) => "Create Child",
            Screen::ChildDetail(_) => "Child Details",
            Screen::Reconciliation => "Reconciliation",
            Screen::Reports => "Reports",
            Screen::QrDisplay(_) => "QR Code",
            Screen::Settings => "Settings",
            Screen::Help => "Help",
            Screen::Confirm(_) => "Confirm",
        }
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
