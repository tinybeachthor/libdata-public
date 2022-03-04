use futures_lite::io::{AsyncRead, AsyncWrite};

use crate::Options;
use crate::io::IO;

/// Handshake stage of the [Protocol].
pub mod handshake;
/// Main stage of the [Protocol].
pub mod main;

/// Init a new [Protocol] with [Options].
#[inline]
pub fn new_protocol<T>(io: T, options: Options)
    -> Protocol<T, handshake::Stage>
where
    T: AsyncWrite + AsyncRead + Send + Unpin + 'static,
{
    Protocol::<T, handshake::Stage>::new(io, options)
}

/// Init a new [Protocol] with default [Options].
#[inline]
pub fn new_protocol_with_defaults<T>(io: T, is_initiator: bool)
    -> Protocol<T, handshake::Stage>
where
    T: AsyncWrite + AsyncRead + Send + Unpin + 'static,
{
    let options = Options::new(is_initiator);
    new_protocol(io, options)
}

/// [Protocol] stage.
pub trait ProtocolStage {}

/// Replication [Protocol].
#[derive(Debug)]
pub struct Protocol<T, S: ProtocolStage> {
    io: IO<T>,
    state: S,
}
