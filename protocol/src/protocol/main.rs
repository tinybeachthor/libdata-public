use anyhow::{Result, anyhow};
use futures_lite::io::{AsyncRead, AsyncWrite};
use futures_lite::stream::Stream;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::io::{self, Error, ErrorKind};
use async_channel::{Receiver, Sender};
use std::collections::VecDeque;
use std::convert::TryInto;

use crate::schema::*;
use crate::message::{Frame, FrameType, ChannelMessage};
use crate::channels::ChannelMap;
use crate::io::IO;
use crate::{noise, Key, DiscoveryKey, Message};

use super::{Protocol, ProtocolStage};

macro_rules! return_error {
    ($msg:expr) => {
        if let Err(e) = $msg {
            return Poll::Ready(Err(e));
        }
    };
}

fn map_channel_err<T>(err: async_channel::SendError<T>) -> Error {
    Error::new(
        ErrorKind::BrokenPipe,
        format!("Cannot forward on channel: {}", err),
    )
}

/// Concurrent channels cap.
pub const CHANNEL_CAP: usize = 1000;

/// Protocol events.
#[derive(PartialEq, Debug)]
pub enum Event {
    /// Emitted when the remote opens a channel that we did not yet open.
    DiscoveryKey(DiscoveryKey),
    /// Channel is established.
    Open(DiscoveryKey),
    /// Channel is closed.
    Close(DiscoveryKey),
    /// A new [Message] received on a channel.
    Message(DiscoveryKey, Message),
}

/// Main stage of [Protocol], contains stage-specific fields.
#[derive(Debug)]
pub struct Stage {
    handshake: Option<noise::HandshakeResult>,
    channels: ChannelMap,
    outbound_rx: Receiver<ChannelMessage>,
    outbound_tx: Sender<ChannelMessage>,
    queued_events: VecDeque<Event>,
}
impl ProtocolStage for Stage {}

impl<T> Protocol<T, Stage>
where
    T: AsyncWrite + AsyncRead + Send + Unpin + 'static,
{
    /// Create a new [Protocol] after completing the handshake.
    pub fn new(mut io: IO<T>, result: Option<noise::HandshakeResult>)
        -> Result<Self>
    {
        // setup core
        if io.options.encrypted && result.is_some() {
            let handshake = result.as_ref().unwrap();
            io.read_state.upgrade_with_handshake(&handshake)?;
            io.write_state.upgrade_with_handshake(&handshake)?;
        }
        io.read_state.set_frame_type(FrameType::Message);

        // setup channels
        let (outbound_tx, outbound_rx) = async_channel::unbounded();

        Ok(Self {
            io,
            state: Stage {
                handshake: result,
                channels: ChannelMap::new(),
                outbound_tx,
                outbound_rx,
                queued_events: VecDeque::new(),
            },
        })
    }

    /// Open a new protocol channel.
    pub async fn open(&mut self, key: Key) -> Result<()> {
        // Create a new channel.
        let channel_handle = self.state.channels.attach_local(key);
        // Safe because attach_local always puts Some(local_id)
        let local_id = channel_handle.local_id().unwrap();
        let discovery_key = *channel_handle.discovery_key();

        // If the channel was already opened from the remote end, verify,
        // and if verification is ok, push a channel open event.
        if channel_handle.is_connected() {
            let (key, remote_capability) =
                self.state.channels.prepare_to_verify(local_id)?;
            self.verify_remote_capability(remote_capability.cloned(), key)?;
            self.queue_event(Event::Open(discovery_key));
        }

        // Tell the remote end about the new channel.
        let capability = self.capability(&key);
        let message = Message::Open(Open {
            discovery_key: discovery_key.to_vec(),
            capability,
        });
        let channel_message = ChannelMessage::new(local_id as u64, message);
        self.io.write_state.queue_frame(Frame::Message(channel_message));
        Ok(())
    }

    /// Close a protocol channel.
    pub async fn close(&mut self, discovery_key: DiscoveryKey) -> Result<()> {
        self.send(&discovery_key, Message::Close(Close {
            discovery_key: discovery_key.to_vec(),
        })).await
    }

    /// Send a [Message] on a channel.
    async fn send(
        &mut self, discovery_key: &DiscoveryKey, msg: Message) -> Result<()>
    {
        match self.state.channels.get(&discovery_key) {
            None => Ok(()),
            Some(channel) => {
                if channel.is_connected() {
                    let local_id = channel.local_id().unwrap();
                    let msg = ChannelMessage::new(local_id as u64, msg);
                    self.state.outbound_tx
                        .send(msg)
                        .await.map_err(map_channel_err)?;
                }
                Ok(())
            },
        }
    }
    /// Send a [Message::Request] on a channel.
    pub async fn request(
        &mut self, discovery_key: &DiscoveryKey, msg: Request) -> Result<()>
    {
        self.send(&discovery_key, Message::Request(msg)).await
    }
    /// Send a [Message::Data] on a channel.
    pub async fn data(
        &mut self, discovery_key: &DiscoveryKey, msg: Data) -> Result<()>
    {
        self.send(&discovery_key, Message::Data(msg)).await
    }

    fn poll_next(
        self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Event>>
    {
        let this = self.get_mut();

        // Drain queued events first
        if let Some(event) = this.state.queued_events.pop_front() {
            return Poll::Ready(Ok(event));
        }

        // Read and process incoming messages
        return_error!(this.poll_inbound_read(cx));

        // Write everything we can write
        return_error!(this.poll_outbound_write(cx));

        // Check if any events are enqueued
        if let Some(event) = this.state.queued_events.pop_front() {
            Poll::Ready(Ok(event))
        } else {
            Poll::Pending
        }
    }

    fn poll_inbound_read(&mut self, cx: &mut Context<'_>) -> Result<()> {
        loop {
            let msg = match self.io.poll_inbound_read(cx) {
                Err(err) => return Err(err),
                Ok(msg) => msg,
            };
            match msg {
                Some(frame) => match frame {
                    Frame::Message(msg) => self.on_inbound_message(msg)?,
                    _ => unreachable!(
                        "May not receive raw frames after handshake"),
                },
                None => return Ok(()),
            };
        }
    }

    fn poll_outbound_write(&mut self, cx: &mut Context<'_>) -> Result<()> {
        loop {
            self.io.poll_outbound_write(cx)?;

            if !self.io.write_state.can_park_frame() {
                return Ok(())
            }

            match Pin::new(&mut self.state.outbound_rx).poll_next(cx) {
                Poll::Ready(Some(message)) => {
                    self.on_outbound_message(&message);
                    let frame = Frame::Message(message);
                    self.io.write_state.park_frame(frame);
                }
                Poll::Ready(None) => unreachable!("Channel closed before end"),
                Poll::Pending => return Ok(())
            }
        }
    }

    fn on_outbound_message(&mut self, message: &ChannelMessage) {
        // If message is close, close the local channel.
        if let ChannelMessage {
            channel,
            message: Message::Close(_),
        } = message
        {
            self.close_local(*channel);
        }
    }

    fn on_inbound_message(
        &mut self,
        channel_message: ChannelMessage,
        ) -> Result<()>
    {
        let (remote_id, message) = channel_message.into_split();
        match remote_id {
            // Id 0 means stream-level
            0 => {},
            // Any other Id is a regular channel message.
            _ => match message {
                Message::Open(msg) => self.on_open(remote_id, msg)?,
                Message::Close(msg) => self.on_close(remote_id, msg)?,
                _ => {
                    // Emit [Event::Message].
                    let discovery_key = self.state.channels
                        .get_remote(remote_id as usize)
                        .map(|remote| remote.discovery_key().clone());
                    if let Some(discovery_key) = discovery_key {
                        self.queue_event(
                            Event::Message(discovery_key.clone(), message));
                    }
                },
            },
        }
        Ok(())
    }

    fn on_open(&mut self, ch: u64, msg: Open) -> Result<()> {
        let discovery_key: DiscoveryKey = parse_key(&msg.discovery_key)?;
        let channel_handle = self.state.channels
            .attach_remote(discovery_key, ch as usize, msg.capability);

        if channel_handle.is_connected() {
            let local_id = channel_handle.local_id().unwrap();
            let (key, remote_capability) =
                self.state.channels.prepare_to_verify(local_id)?;
            self.verify_remote_capability(remote_capability.cloned(), key)?;
            self.queue_event(Event::Open(discovery_key));
        } else {
            self.queue_event(Event::DiscoveryKey(discovery_key));
        }

        Ok(())
    }

    fn close_local(&mut self, local_id: u64) {
        let channel = self.state.channels.get_local(local_id as usize);
        if let Some(channel) = channel {
            let discovery_key = *channel.discovery_key();
            self.state.channels.remove(&discovery_key);
            self.queue_event(Event::Close(discovery_key));
        }
    }

    fn on_close(&mut self, remote_id: u64, msg: Close) -> Result<()> {
        let remote = self.state.channels.get_remote(remote_id as usize);
        if let Some(channel_handle) = remote {
            let discovery_key = *channel_handle.discovery_key();
            if msg.discovery_key == discovery_key {
                self.state.channels.remove(&discovery_key);
                self.queue_event(Event::Close(discovery_key));
            }
        }
        Ok(())
    }

    fn queue_event(&mut self, event: Event) {
        self.state.queued_events.push_back(event);
    }

    fn capability(&self, key: &[u8]) -> Option<Vec<u8>> {
        match self.state.handshake.as_ref() {
            Some(handshake) => handshake.capability(key),
            None => None,
        }
    }

    fn verify_remote_capability(
        &self,
        capability: Option<Vec<u8>>,
        key: &[u8],
        ) -> Result<()>
    {
        match self.state.handshake.as_ref() {
            Some(handshake) => handshake
                .verify_remote_capability(capability, key)
                .map_err(|err| anyhow!(err)),
            None => Err(anyhow!(Error::new(
                ErrorKind::PermissionDenied,
                "Missing handshake state for capability verification",
            ))),
        }
    }
}

impl<T> Stream for Protocol<T, Stage>
where
    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Item = Result<Event>;
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        ) -> Poll<Option<Self::Item>>
    {
        Self::poll_next(self, cx).map(Some)
    }
}

fn parse_key(key: &[u8]) -> io::Result<[u8; 32]> {
    key.try_into().map_err(
        |_| io::Error::new(
            io::ErrorKind::InvalidInput,
            "Key must be 32 bytes long"))
}
