#![forbid(unsafe_code, bad_style, nonstandard_style, future_incompatible)]
#![forbid(rust_2018_idioms, rust_2021_compatibility)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![cfg_attr(test, allow(dead_code))]
#![cfg_attr(test, deny(warnings))]

//! Abstract interface to implement random-access storage.
//! This module forms a shared interface for reading and writing bytes
//! to different backends.
//! By having a shared interface, it means implementations can easily be swapped,
//! depending on the environment.

/// The `RandomAccess` trait allows for reading from and writing to a
/// randomly accessible storage of bytes.
#[async_trait::async_trait]
pub trait RandomAccess {
  /// An error.
  type Error;

  /// Write bytes at an offset to the backend.
  async fn write(
    &mut self,
    offset: u64,
    data: &[u8],
  ) -> Result<(), Self::Error>;

  /// Read a sequence of bytes at an offset from the backend.
  async fn read(
    &mut self,
    offset: u64,
    length: u64,
  ) -> Result<Vec<u8>, Self::Error>;
}
