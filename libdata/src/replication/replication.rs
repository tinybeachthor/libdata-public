use anyhow::{Result, anyhow};
use std::fmt::Debug;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::collections::HashMap;
use std::future::Future;
use futures_lite::io::{AsyncRead, AsyncWrite};
use futures_lite::stream::{Stream, StreamExt};
use async_channel;

use protocol::{new_protocol, Protocol, Message};
use protocol::main::{Stage, Event as ProtocolEvent};
use crate::{DiscoveryKey, discovery_key};
use crate::replication::{
    Options, ReplicaTrait, Request, Data, DataOrRequest,
    Command, ReplicationHandle,
};

/// [Replication] event.
#[derive(Debug)]
pub enum Event {
    Command(Command),
    Event(Result<ProtocolEvent>),
}

/// Replication protocol main abstraction:
/// handle handshake, multiplexing, failures.
///
/// Concrete behavior is specified in [ReplicaTrait].
pub struct Replication<T: 'static>
where
    T: AsyncWrite + AsyncRead + Send + Unpin,
{
    protocol: Protocol<T, Stage>,
    command_rx: async_channel::Receiver<Command>,
    replicas: HashMap<DiscoveryKey, Box<dyn ReplicaTrait + Send>>,
}
impl<T: 'static> Debug for Replication<T>
where
    T: AsyncWrite + AsyncRead + Send + Unpin,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>)
        -> Result<(), std::fmt::Error>
    {
        write!(fmt, "Replication")
    }
}
impl<T: 'static> Replication<T>
where
    T: AsyncWrite + AsyncRead + Send + Unpin,
{
    /// Create `Replication` and wait for protocol handshake.
    pub async fn new(stream: T, is_initiator: bool)
        -> Result<(Self, ReplicationHandle)>
    {
        Self::with_options(stream, Options {
            is_initiator,
            ..Options::default()
        }).await
    }

    /// Create `Replication` with [Options] and wait for protocol handshake.
    pub async fn with_options(stream: T, options: Options)
        -> Result<(Self, ReplicationHandle)>
    {
        let (tx, rx) = async_channel::unbounded();
        let handle = ReplicationHandle { tx };

        let handshake = new_protocol(stream, options);
        let protocol = handshake.handshake().await?;

        let replication = Self {
            protocol,
            command_rx: rx,
            replicas: HashMap::new(),
        };

        Ok((replication, handle))
    }

    /// Run the replication loop to completion.
    pub async fn run(self) -> Result<()> {
        let on_discovery = |_| async move { Ok(()) };
        self.run_with_discovery_hook(on_discovery).await
    }
    /// Run the replication loop to completion
    /// with an `on_discovery` hook: handle [ProtocolEvent::DiscoveryKey].
    pub async fn run_with_discovery_hook<F>(
        mut self,
        on_discovery: impl Fn(DiscoveryKey) -> F,
        ) -> Result<()>
    where
        F: Future<Output=Result<()>>,
    {
        loop {
            match self.next().await.unwrap() {
                Event::Command(cmd) => {
                    if !self.handle_command(cmd).await? {
                        return Ok(())
                    }
                },
                Event::Event(event) => {
                    let on_discovery = |discovery| on_discovery(discovery);
                    if !self.handle_event(event, on_discovery).await? {
                        return Ok(())
                    }
                },
            };
        }
    }
    async fn handle_command(&mut self, command: Command) -> Result<bool> {
        #[cfg(test)] println!("handle_command {:?}", command);

        match command {
            Command::Open(key, replica) => {
                let discovery = discovery_key(&key.to_bytes());
                self.replicas.insert(discovery, replica);
                self.protocol.open(key.to_bytes()).await?;
                Ok(true)
            },
            Command::ReOpen(key) => {
                self.replica_on_open(&key).await?;
                Ok(true)
            },
            Command::Close(key) => {
                self.protocol
                    .close(key)
                    .await?;
                self.replicas.remove(&key);
                Ok(true)
            },
            Command::Quit() => {
                let mut is_error = false;
                for (_, replica) in self.replicas.iter_mut() {
                    is_error |= replica.on_close().await.is_err();
                }
                return match is_error {
                    true => Err(anyhow!("Quit before replication finished.")),
                    false => Ok(false),
                }
            },
        }
    }
    async fn handle_event<F>(
        &mut self,
        event: Result<ProtocolEvent>,
        on_discovery: impl FnOnce(DiscoveryKey) -> F,
        ) -> Result<bool>
    where
        F: Future<Output=Result<()>>,
    {
        #[cfg(test)] println!("handle_event {:?}", event);

        let msg = match event {
            Ok(msg) => msg,
            Err(err) => {
                let mut is_error = false;
                for (_, replica) in self.replicas.iter_mut() {
                    is_error |= replica.on_close().await.is_err();
                }
                return match is_error {
                    true => Err(err),
                    false => Ok(false),
                }
            },
        };

        match msg {
            ProtocolEvent::DiscoveryKey(discovery) => {
                on_discovery(discovery).await?;
            },
            ProtocolEvent::Open(discovery) => {
                self.replica_on_open(&discovery).await?;
            },
            ProtocolEvent::Close(discovery) => {
                self.replica_on_close(&discovery).await?;
            },
            ProtocolEvent::Message(discovery, msg) => match msg {
                Message::Request(request) => {
                    self.replica_on_request(&discovery, request).await?;
                },
                Message::Data(data) => {
                    self.replica_on_data(&discovery, data).await?;
                },
                _ => {},
            },
        };
        Ok(true)
    }

    async fn replica_on_open(
        &mut self, key: &DiscoveryKey) -> Result<()>
    {
        if let Some(replica) = self.replicas.get_mut(key) {
            let request = replica.on_open().await?;
            if let Some(request) = request {
                self.protocol
                    .request(key, request)
                    .await?;
            }
        }
        Ok(())
    }

    async fn replica_on_close(
        &mut self, key: &DiscoveryKey) -> Result<()>
    {
        if let Some(replica) = self.replicas.get_mut(key) {
            replica.on_close().await?;
        }
        self.replicas.remove(key);
        Ok(())
    }

    async fn replica_on_request(
        &mut self, key: &DiscoveryKey, request: Request) -> Result<()>
    {
        if let Some(replica) = self.replicas.get_mut(key) {
            let msg = replica.on_request(request).await?;
            match msg {
                Some(DataOrRequest::Data(data)) =>
                    self.protocol.data(key, data).await?,
                Some(DataOrRequest::Request(request)) =>
                    self.protocol.request(key, request).await?,
                None => {},
            };
        }
        Ok(())
    }

    async fn replica_on_data(
        &mut self, key: &DiscoveryKey, data: Data) -> Result<()>
    {
        if let Some(replica) = self.replicas.get_mut(key) {
            let request = replica.on_data(data).await?;
            if let Some(request) = request {
                self.protocol
                    .request(key, request)
                    .await?;
            }
        }
        Ok(())
    }
}
impl<T: 'static> Stream for Replication<T>
where
    T: AsyncWrite + AsyncRead + Send + Unpin,
{
    type Item = Event;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        ) -> Poll<Option<Self::Item>>
    {
        let this = self.get_mut();

        if let Poll::Ready(Some(t)) = this.command_rx.poll_next(cx) {
            return Poll::Ready(Some(Event::Command(t)));
        }
        if let Poll::Ready(Some(t)) = this.protocol.poll_next(cx) {
            return Poll::Ready(Some(Event::Event(t)));
        }
        Poll::Pending
    }
}
