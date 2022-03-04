use std::io::{Error, ErrorKind, Result};
use std::pin::Pin;
use std::time::Duration;
use std::task::{Context, Poll};
use std::future::Future;
use futures_lite::io::AsyncRead;
use futures_timer::Delay;

use crate::message::{Frame, FrameType};
use crate::noise::{Cipher, HandshakeResult};
use crate::MAX_MESSAGE_SIZE;

const READ_BUF_INITIAL_SIZE: usize = 1024 * 128;

#[derive(Debug)]
pub struct ReadState {
    /// The read buffer.
    buf: Vec<u8>,
    /// The start of the not-yet-processed byte range in the read buffer.
    start: usize,
    /// The end of the not-yet-processed byte range in the read buffer.
    end: usize,
    /// The logical state of the reading (either header or body).
    step: Step,
    /// The timeout after which the connection is closed.
    timeout: Option<Delay>,
    /// Timeout duration.
    timeout_duration: Option<Duration>,
    /// Optional encryption cipher.
    cipher: Option<Cipher>,
    /// The frame type to be passed to the decoder.
    frame_type: FrameType,
}

impl ReadState {
    pub fn new(timeout_ms: Option<u64>) -> Self {
        let timeout_duration = timeout_ms.map(Duration::from_millis);
        Self {
            buf: vec![0u8; READ_BUF_INITIAL_SIZE as usize],
            start: 0,
            end: 0,
            step: Step::Header,
            timeout: timeout_duration.map(Delay::new),
            timeout_duration,
            cipher: None,
            frame_type: FrameType::Raw,
        }
    }
}

#[derive(Debug)]
enum Step {
    Header,
    Body { header_len: usize, body_len: usize },
}

impl ReadState {
    pub fn upgrade_with_handshake(&mut self, handshake: &HandshakeResult) -> Result<()> {
        let mut cipher = Cipher::from_handshake_rx(handshake)?;
        cipher.apply(&mut self.buf[self.start..self.end]);
        self.cipher = Some(cipher);
        Ok(())
    }

    pub fn set_frame_type(&mut self, frame_type: FrameType) {
        self.frame_type = frame_type;
    }

    pub fn poll_reader<R>(
        &mut self,
        cx: &mut Context<'_>,
        mut reader: &mut R,
    ) -> Poll<Result<Frame>>
    where
        R: AsyncRead + Unpin,
    {
        loop {
            if let Some(result) = self.process() {
                return Poll::Ready(result);
            }

            let n = match Pin::new(&mut reader).poll_read(cx, &mut self.buf[self.end..]) {
                Poll::Ready(Ok(n)) if n > 0 => n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                // If the reader is pending, poll the timeout.
                Poll::Pending | Poll::Ready(Ok(_)) => {
                    // Return Pending if the timeout is pending, or an error if the
                    // timeout expired (i.e. returned Poll::Ready).
                    return match self.timeout.as_mut() {
                        None => Poll::Pending,
                        Some(mut timeout) => match Pin::new(&mut timeout).poll(cx) {
                            Poll::Pending => Poll::Pending,
                            Poll::Ready(_) => Poll::Ready(Err(
                                    Error::new(ErrorKind::TimedOut, "Remote timed out"))),
                        },
                    }
                }
            };

            let end = self.end + n;
            if let Some(ref mut cipher) = self.cipher {
                cipher.apply(&mut self.buf[self.end..end]);
            }
            self.end = end;

            // reset timeout
            match self.timeout_duration {
                None => None,
                Some(timeout_duration) =>
                    self.timeout.as_mut().map(|t| t.reset(timeout_duration)),
            };
        }
    }

    fn cycle_buf_if_needed(&mut self) {
        // TODO: It would be great if we wouldn't have to allocate here.
        if self.end == self.buf.len() {
            let temp = self.buf[self.start..self.end].to_vec();
            let len = temp.len();
            self.buf[..len].copy_from_slice(&temp[..]);
            self.end = len;
            self.start = 0;
        }
    }

    fn process(&mut self) -> Option<Result<Frame>> {
        if self.start == self.end {
            return None;
        }
        loop {
            match self.step {
                Step::Header => {
                    let mut body_len = 0;
                    let header_len = varinteger::decode(
                        &self.buf[self.start..self.end], &mut body_len);

                    let body_len = body_len as usize;
                    if body_len > MAX_MESSAGE_SIZE as usize {
                        return Some(Err(Error::new(
                            ErrorKind::InvalidData,
                            "Message length above max allowed size",
                        )));
                    }
                    self.step = Step::Body {
                        header_len,
                        body_len,
                    };
                }
                Step::Body {
                    header_len,
                    body_len,
                } => {
                    let message_len = header_len + body_len;
                    if message_len > self.buf.len() {
                        self.buf.resize(message_len, 0u8);
                    }
                    if (self.end - self.start) < message_len {
                        self.cycle_buf_if_needed();
                        return None;
                    } else {
                        let range = self.start + header_len..self.start + message_len;
                        let frame = Frame::decode(&self.buf[range], &self.frame_type);
                        self.start += message_len;
                        self.step = Step::Header;
                        return Some(frame);
                    }
                }
            }
        }
    }
}
