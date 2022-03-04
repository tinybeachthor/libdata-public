#![forbid(unsafe_code, bad_style, nonstandard_style, future_incompatible)]
#![forbid(rust_2018_idioms, rust_2021_compatibility)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![cfg_attr(test, allow(dead_code))]
#![cfg_attr(test, deny(warnings))]

//! ## Introduction
//! Datacore is a secure, append-only, single-writer log.
//! It is meant for sharing large datasets and streams of real time data.
//! The primary way to use this crate is through the [Core] struct.
//!
//! ## Example
//! ```rust
//! # use async_std::prelude::*;
//! # use futures_lite::future::FutureExt;
//!
//! use random_access_memory::RandomAccessMemory;
//! use datacore::{Core, generate_keypair};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! # async_std::task::block_on(async {
//! let keypair = generate_keypair();
//! let mut core = Core::new(
//!     RandomAccessMemory::new(1024),
//!     RandomAccessMemory::new(1024),
//!     RandomAccessMemory::new(1024),
//!     keypair.public, Some(keypair.secret)
//!     ).await?;
//!
//! core.append(b"hello", None).await?;
//! core.append(b"world", None).await?;
//!
//! assert_eq!(core.len(), 2);
//! assert_eq!(
//!     core.get(0).await?.unwrap().0,
//!     b"hello");
//! assert_eq!(
//!     core.get(1).await?.unwrap().0,
//!     b"world");
//! # Ok(())
//! # })
//! # }
//! ```

mod block;
mod store_data;
mod store_blocks;
mod store_state;
mod merkle_tree_stream;
mod keys;
mod hash;
mod merkle;
mod core;

pub use random_access_storage::RandomAccess;
pub use block::{Signature, BlockSignature, Block, SIGNATURE_LENGTH};
pub use keys::{
    Keypair, PublicKey, SecretKey,
    generate_keypair, sign, verify
};
pub use hash::Hash;
pub use merkle::{Merkle, Node, NodeTrait};
pub use self::core::{Core, MAX_CORE_LENGTH, MAX_BLOCK_SIZE};
