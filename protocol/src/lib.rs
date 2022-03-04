#![forbid(unsafe_code, bad_style, nonstandard_style, future_incompatible)]
#![forbid(rust_2018_idioms, rust_2021_compatibility)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![cfg_attr(test, allow(dead_code))]
#![cfg_attr(test, deny(warnings))]

//! Replication protocol for hypercore feeds.

mod options;
mod channels;
mod duplex;
mod message;
mod io;
mod util;
mod noise;
mod protocol;

/// The wire messages used by the protocol.
#[allow(missing_docs)]
pub mod schema {
    include!(concat!(env!("OUT_DIR"), "/hypercore.schema.rs"));
}

/// Maximum size of a `Message`.
// 4MB is the max wire message size (will be much smaller usually).
pub const MAX_MESSAGE_SIZE: u64 = 1024 * 1024 * 4;

/// Public key (32 bytes).
pub type Key = [u8; 32];
/// Remote public key (32 bytes).
pub type RemotePublicKey = [u8; 32];
/// Discovery key (32 bytes).
pub type DiscoveryKey = [u8; 32];

pub use options::Options;
pub use duplex::Duplex;
pub use message::Message;
pub use util::discovery_key;
pub use crate::protocol::{
    new_protocol, new_protocol_with_defaults,
    Protocol, handshake, main,
};
