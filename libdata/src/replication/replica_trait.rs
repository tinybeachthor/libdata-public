use anyhow::Result;
use async_trait::async_trait;

pub use protocol::schema::{Data, Request};

/// Either [Data] or [Request].
#[derive(Debug)]
pub enum DataOrRequest {
    /// [Data].
    Data(Data),
    /// [Request].
    Request(Request),
}

/// ReplicaTrait describes the behavior of [Replication].
///
/// [Replication]: super::Replication
#[async_trait]
pub trait ReplicaTrait {
    /// Called on connection opened.
    /// Optionally return a [Request].
    async fn on_open(&mut self)
        -> Result<Option<Request>>;

    /// Called on new [Request] received.
    /// Optionally return [DataOrRequest] to send back.
    async fn on_request(&mut self, request: Request)
        -> Result<Option<DataOrRequest>>;

    /// Called on new [Data] received.
    /// Optionally return a new [Request].
    async fn on_data(&mut self, data: Data)
        -> Result<Option<Request>>;

    /// Called on connection close (possibly abnormal).
    /// Return `Ok` if this replica was synced correctly.
    async fn on_close(&mut self)
        -> Result<()>;
}
