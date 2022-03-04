/// Default keepalive interval (in milliseconds)
pub const DEFAULT_KEEPALIVE: u64 = 10_000;

/// Options for a Protocol instance.
#[derive(Debug)]
pub struct Options {
    /// Whether this peer initiated the IO connection for this protocol.
    pub is_initiator: bool,
    /// Enable or disable the handshake.
    /// Disabling the handshake will also disable capability verification.
    /// Don't disable this if you're not 100% sure you want this.
    pub noise: bool,
    /// Enable or disable transport encryption.
    pub encrypted: bool,
    /// Keepalive time in milliseconds or `None` for no timeout.
    pub keepalive_ms: Option<u64>,
}

impl Options {
    /// Create with default options.
    pub fn new(is_initiator: bool) -> Self {
        Self {
            is_initiator,
            ..Self::default()
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            is_initiator: false,
            noise: true,
            encrypted: true,
            keepalive_ms: Some(DEFAULT_KEEPALIVE),
        }
    }
}
