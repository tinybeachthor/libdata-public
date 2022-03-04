use anyhow::{Result, anyhow};
use std::fmt::Debug;
use async_channel;

use crate::{DiscoveryKey, PublicKey, discovery_key};
use crate::replication::ReplicaTrait;

/// [Replication] command.
pub enum Command {
    /// Open a new replica.
    Open(PublicKey, Box<dyn ReplicaTrait + Send>),
    /// Re-open a replica.
    ReOpen(DiscoveryKey),
    /// Close a replica.
    Close(DiscoveryKey),
    /// End the [Replication].
    Quit(),
}
impl Debug for Command {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>)
        -> Result<(), std::fmt::Error>
    {
        match self {
            Self::Open(key, _) =>
                write!(fmt, "Command::Open({:?})", key),
            Self::ReOpen(key) =>
                write!(fmt, "Command::ReOpen({:?})", key),
            Self::Close(key) =>
                write!(fmt, "Command::Close({:?})", key),
            Self::Quit() =>
                write!(fmt, "Command::Quit()"),
        }
    }
}

/// [Replication] handle.
#[derive(Debug, Clone)]
pub struct ReplicationHandle {
    pub(crate) tx: async_channel::Sender<Command>,
}
impl ReplicationHandle {
    /// Open a new channel with [ReplicaTrait].
    pub async fn open(
        &mut self,
        key: &PublicKey,
        replica: Box<dyn ReplicaTrait + Send>,
        ) -> Result<()>
    {
        let cmd = Command::Open(key.clone(), replica);
        self.tx.send(cmd)
            .await.map_err(|_| anyhow!("Error sending command."))
    }

    /// Reopen a replica.
    pub async fn reopen(&mut self, key: &PublicKey) -> Result<()> {
        let cmd = Command::ReOpen(discovery_key(key.as_bytes()));
        self.tx.send(cmd)
            .await.map_err(|_| anyhow!("Error sending command."))
    }

    /// Close a channel by [DiscoveryKey].
    pub async fn close(&mut self, key: DiscoveryKey) -> Result<()> {
        let cmd = Command::Close(key);
        self.tx.send(cmd)
            .await.map_err(|_| anyhow!("Error sending command."))
    }

    /// End the [Replication].
    pub async fn quit(&mut self) -> Result<()> {
        let cmd = Command::Quit();
        self.tx.send(cmd)
            .await.map_err(|_| anyhow!("Error sending command."))
    }
}
