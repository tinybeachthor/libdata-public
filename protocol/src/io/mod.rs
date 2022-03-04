mod reader;
mod writer;

use anyhow::{Result, anyhow};
use std::task::{Context, Poll};
use futures_lite::io::{AsyncRead, AsyncWrite};

use crate::Options;
use crate::message::{Frame, EncodeError};
use self::reader::ReadState;
use self::writer::WriteState;

#[derive(Debug)]
pub struct IO<T> {
    io: T,
    pub options: Options,
    pub read_state: ReadState,
    pub write_state: WriteState,
}

impl<T> IO<T>
where
    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    pub fn new(io: T, options: Options) -> Self {
        let keepalive_ms = options.keepalive_ms;
        Self {
            io,
            options,
            read_state: ReadState::new(keepalive_ms),
            write_state: WriteState::new(),
        }
    }

    /// Poll for inbound messages and process them.
    pub fn poll_inbound_read(
        &mut self,
        cx: &mut Context<'_>,
        ) -> Result<Option<Frame>>
    {
        let msg = self.read_state.poll_reader(cx, &mut self.io);
        return match msg {
            Poll::Ready(Ok(message)) => Ok(Some(message)),
            Poll::Ready(Err(e)) => Err(anyhow!(e)),
            Poll::Pending => Ok(None),
        }
    }

    /// Poll for outbound messages and write them.
    pub fn poll_outbound_write(&mut self, cx: &mut Context<'_>) -> Result<()>
    {
        let poll = self.write_state.poll_send(cx, &mut self.io);
        if let Poll::Ready(Err(e)) = poll {
            return Err(anyhow!(e));
        }
        return Ok(());
    }

    pub fn queue_frame_direct(&mut self, body: Vec<u8>)
        -> std::result::Result<bool, EncodeError>
    {
        let frame = Frame::Raw(body);
        self.write_state.try_queue_direct(&frame)
    }
}
