#![cfg_attr(test, allow(dead_code))]

use anyhow::Result;
use std::future::Future;
use futures_lite::io::{AsyncRead, AsyncWrite};
use futures_lite::stream::StreamExt;
use async_std::task;
use async_std::net::TcpStream;
use sluice::pipe::{PipeReader, PipeWriter, pipe};

use protocol::{
    Options, Duplex,
    Protocol, handshake, main,
    new_protocol, new_protocol_with_defaults,
};

pub fn create_duplex_pair_memory()
    -> (Duplex<PipeReader, PipeWriter>, Duplex<PipeReader, PipeWriter>)
{
    let (ar, bw) = pipe();
    let (br, aw) = pipe();

    (Duplex::new(ar, aw), Duplex::new(br, bw))
}

pub type MemoryProtocol =
    Protocol<Duplex<PipeReader, PipeWriter>, handshake::Stage>;
pub fn create_pair_memory()
    -> Result<(MemoryProtocol, MemoryProtocol)>
{
    create_pair_memory_keepalive(Some(1_000))
}
pub fn create_pair_memory_keepalive(keepalive_ms: Option<u64>)
    -> Result<(MemoryProtocol, MemoryProtocol)>
{
    let (a, b) = create_duplex_pair_memory();
    let b = new_protocol(b, Options {
        is_initiator: false,
        keepalive_ms,
        ..Options::default()
    });
    let a = new_protocol(a, Options {
        is_initiator: true,
        keepalive_ms,
        ..Options::default()
    });
    Ok((a, b))
}

pub async fn establish<T>(
    a: Protocol<T, handshake::Stage>,
    b: Protocol<T, handshake::Stage>,
) -> (Protocol<T, main::Stage>, Protocol<T, main::Stage>)
where
    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    let task_a = task::spawn(async move {
        a.handshake().await.unwrap()
    });
    let task_b = task::spawn(async move {
        b.handshake().await.unwrap()
    });
    let a = task_a.await;
    let b = task_b.await;
    (a, b)
}

pub fn next_event<T>(
    mut proto: Protocol<T, main::Stage>,
) -> impl Future<Output = (Result<main::Event>, Protocol<T, main::Stage>)>
where
    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    let task = task::spawn(async move {
        let e1 = proto.next().await;
        let e1 = e1.unwrap();
        (e1, proto)
    });
    task
}

pub type TcpProtocol = Protocol<TcpStream, handshake::Stage>;
pub async fn create_pair_tcp() -> Result<(TcpProtocol, TcpProtocol)> {
    let (stream_a, stream_b) = tcp::pair().await?;
    let b = new_protocol_with_defaults(stream_b, false);
    let a = new_protocol_with_defaults(stream_a, true);
    Ok((a, b))
}

pub mod tcp {
    use async_std::net::{TcpListener, TcpStream};
    use async_std::prelude::*;
    use async_std::task;
    use std::io::{Error, ErrorKind, Result};

    pub async fn pair() -> Result<(TcpStream, TcpStream)> {
        let address = "localhost:9999";
        let listener = TcpListener::bind(&address).await?;
        let mut incoming = listener.incoming();

        let connect_task = task::spawn(async move {
            TcpStream::connect(&address).await
        });

        let server_stream = incoming.next().await;
        let server_stream = server_stream.ok_or_else(
            || Error::new(ErrorKind::Other, "Stream closed"))?;
        let server_stream = server_stream?;
        let client_stream = connect_task.await?;
        Ok((server_stream, client_stream))
    }
}
