//! Clock listeners that get notified on clock ticks and other events
#![warn(missing_docs)]
#![warn(unsafe_code)]
#[cfg(feature = "async_tokio")]
use tokio::sync::mpsc::Sender;

/// A ClockListener represents an individual subscriber to Clock ticks
/// and other events.
pub struct ClockListener {
    /// The transmit channel to use to send events to the listener
    pub tx: Sender<()>,
}

impl ClockListener {
    /// tick is called on the clock pulse
    pub fn tick() {}
}
