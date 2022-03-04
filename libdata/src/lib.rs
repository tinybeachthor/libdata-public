#![forbid(unsafe_code, bad_style, nonstandard_style, future_incompatible)]
#![forbid(rust_2018_idioms, rust_2021_compatibility)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![cfg_attr(test, allow(dead_code))]
#![cfg_attr(test, deny(warnings))]

//! Libdata re-exports public interface from [datacore],
//! defines async [CoreIterator],
//! defines interface for managing collection of [Cores],
//! and specifies [replication] over [protocol].

pub use datacore::{
    Core, RandomAccess, BlockSignature, Signature,
    MAX_CORE_LENGTH,
};

mod key;
pub use key::{
    Keypair, PublicKey, SecretKey, DiscoveryKey,
    generate_keypair, derive_keypair, discovery_key,
};

mod iter;
pub use iter::CoreIterator;

mod cores;
pub use cores::Cores;

pub mod replication;
