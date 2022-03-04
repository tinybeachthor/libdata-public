use std::fmt;
use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::collections::VecDeque;
use futures_lite::{ready, AsyncWrite};

use crate::message::{EncodeError, Encoder, Frame};
use crate::noise::{Cipher, HandshakeResult};

const BUF_SIZE: usize = 1024 * 64;

#[derive(Debug)]
pub enum Step {
    Flushing,
    Writing,
    Processing,
}

pub struct WriteState {
    queue: VecDeque<Frame>,
    buf: Vec<u8>,
    current_frame: Option<Frame>,
    start: usize,
    end: usize,
    cipher: Option<Cipher>,
    step: Step,
}

impl fmt::Debug for WriteState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WriteState")
            .field("queue (len)", &self.queue.len())
            .field("step", &self.step)
            .field("buf (len)", &self.buf.len())
            .field("current_frame", &self.current_frame)
            .field("start", &self.start)
            .field("end", &self.end)
            .field("cipher", &self.cipher.is_some())
            .finish()
    }
}

impl WriteState {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            buf: vec![0u8; BUF_SIZE],
            current_frame: None,
            start: 0,
            end: 0,
            cipher: None,
            step: Step::Processing,
        }
    }

    pub fn queue_frame<F>(&mut self, frame: F)
    where
        F: Into<Frame>,
    {
        self.queue.push_back(frame.into())
    }

    pub fn try_queue_direct<T: Encoder>(
        &mut self,
        frame: &T,
    ) -> std::result::Result<bool, EncodeError> {
        let len = frame.encoded_len();
        if self.buf.len() < len {
            self.buf.resize(len, 0u8);
        }
        if len > self.remaining() {
            return Ok(false);
        }
        let len = frame.encode(&mut self.buf[self.end..])?;
        self.advance(len);
        Ok(true)
    }

    pub fn can_park_frame(&self) -> bool {
        self.current_frame.is_none()
    }

    pub fn park_frame<F>(&mut self, frame: F)
    where
        F: Into<Frame>,
    {
        if self.current_frame.is_none() {
            self.current_frame = Some(frame.into())
        }
    }

    fn advance(&mut self, n: usize) {
        let end = self.end + n;
        if let Some(ref mut cipher) = self.cipher {
            cipher.apply(&mut self.buf[self.end..end]);
        }
        self.end = end;
    }

    pub fn upgrade_with_handshake(&mut self, handshake: &HandshakeResult) -> Result<()> {
        let cipher = Cipher::from_handshake_tx(handshake)?;
        self.cipher = Some(cipher);
        Ok(())
    }
    fn remaining(&self) -> usize {
        self.buf.len() - self.end
    }

    fn pending(&self) -> usize {
        self.end - self.start
    }

    pub fn poll_send<W>(&mut self, cx: &mut Context<'_>, mut writer: &mut W) -> Poll<Result<()>>
    where
        W: AsyncWrite + Unpin,
    {
        loop {
            self.step = match self.step {
                Step::Processing => {
                    if self.current_frame.is_none() && !self.queue.is_empty() {
                        self.current_frame = self.queue.pop_front();
                    }

                    if let Some(frame) = self.current_frame.take() {
                        if !self.try_queue_direct(&frame)? {
                            self.current_frame = Some(frame);
                        }
                    }
                    if self.pending() == 0 {
                        return Poll::Ready(Ok(()));
                    }
                    Step::Writing
                }
                Step::Writing => {
                    let n = ready!(
                        Pin::new(&mut writer).poll_write(cx, &self.buf[self.start..self.end])
                    )?;
                    self.start += n;
                    if self.start == self.end {
                        self.start = 0;
                        self.end = 0;
                    }
                    Step::Flushing
                }
                Step::Flushing => {
                    ready!(Pin::new(&mut writer).poll_flush(cx))?;
                    Step::Processing
                }
            }
        }
    }
}
