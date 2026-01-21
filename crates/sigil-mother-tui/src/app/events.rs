//! Event handling for the TUI

use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use tokio::sync::mpsc;

/// Application events
#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard input
    Key(KeyEvent),
    /// Terminal tick (for animations)
    Tick,
    /// Disk inserted
    DiskInserted { child_id: String },
    /// Disk removed
    DiskRemoved,
    /// Session timeout warning
    SessionWarning { seconds_remaining: u64 },
    /// Session expired
    SessionExpired,
    /// Operation completed
    OperationComplete { success: bool, message: String },
}

/// Event handler that runs in a separate task
pub struct EventHandler {
    /// Sender for events
    sender: mpsc::UnboundedSender<Event>,
    /// Receiver for events
    receiver: mpsc::UnboundedReceiver<Event>,
    /// Tick rate for animations
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver,
            tick_rate,
        }
    }

    /// Get a clone of the sender for other tasks to send events
    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.sender.clone()
    }

    /// Try to receive the next event (non-blocking)
    pub fn try_recv(&mut self) -> Option<Event> {
        self.receiver.try_recv().ok()
    }

    /// Receive the next event (blocking with timeout)
    pub async fn recv(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }

    /// Poll for keyboard events with timeout
    pub fn poll_keyboard(&self) -> std::io::Result<Option<KeyEvent>> {
        if event::poll(self.tick_rate)? {
            if let CrosstermEvent::Key(key) = event::read()? {
                return Ok(Some(key));
            }
        }
        Ok(None)
    }
}

/// Start the event handler task
pub async fn start_event_loop(
    tick_rate: Duration,
) -> (mpsc::UnboundedSender<Event>, mpsc::UnboundedReceiver<Event>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let tx_clone = tx.clone();

    // Spawn tick generator
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tick_rate);
        loop {
            interval.tick().await;
            if tx_clone.send(Event::Tick).is_err() {
                break;
            }
        }
    });

    (tx, rx)
}
