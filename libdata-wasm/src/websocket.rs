//! WASM WebSocket.

use std::fmt::Debug;
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsError;
use ws_stream_wasm::{WsMeta, WsEvent, WsStreamIo};
use futures_lite::future::race;
use futures_lite::stream::StreamExt;
use futures_lite::io::{AsyncRead, AsyncWrite};
use fluvio_wasm_timer::Delay;
use async_io_stream::IoStream;
use pharos::{self, Observable};

use libdata::PublicKey;
use libdata::replication::{
    Replication, ReplicationHandle, Options, ReplicaTrait, CoreReplica};
use crate::core::CoreWasm;
use crate::keys::PublicKeyWasm;

trait AsyncReadWrite: AsyncRead + AsyncWrite {}

/// WASM wrapper for a vector of replicas.
#[wasm_bindgen]
pub struct ReplicasWasm {
    replicas: Vec<(PublicKey, Box<dyn ReplicaTrait + Send>)>,
}
#[wasm_bindgen]
impl ReplicasWasm {
    /// Returns a new empty [ReplicasWasm].
    pub fn new_empty() -> ReplicasWasm {
        let replicas = vec![];
        ReplicasWasm { replicas }
    }
    /// Add a new replica for [CoreWasm].
    pub fn add_core(&mut self, core: &CoreWasm) {
        let public_key = core.public_key_inner().clone();
        let core = core.clone_inner();
        let replica = Box::new(CoreReplica::new(core));
        self.replicas.push(
            (public_key, replica as Box<dyn ReplicaTrait + Send>))
    }
}
impl ReplicasWasm {
    /// Wrap.
    pub fn new(replicas: Vec<(PublicKey, Box<dyn ReplicaTrait + Send>)>)
        -> Self
    {
        Self { replicas }
    }
    /// Unwrap.
    pub fn take(self) -> Vec<(PublicKey, Box<dyn ReplicaTrait + Send>)> {
        self.replicas
    }
}
impl Debug for ReplicasWasm {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>)
        -> Result<(), std::fmt::Error>
    {
        write!(fmt, "ReplicasWasm")
    }
}

/// WASM wrapper for [ReplicationHandle].
#[wasm_bindgen]
#[derive(Debug)]
pub struct ReplicationHandleWasm {
    handle: ReplicationHandle,
}
#[wasm_bindgen]
impl ReplicationHandleWasm {
    /// Open replicas on the replication.
    pub async fn open_replicas(
        mut self,
        replicas: ReplicasWasm,
        ) -> Result<ReplicationHandleWasm, JsError>
    {
        for (public_key, replica) in replicas.take().into_iter() {
            self.handle
                .open(&public_key, replica)
                .await.map_err(|_| JsError::new("Error opening replica."))?;
        }
        Ok(self)
    }

    /// Re-open a replica on the replication.
    pub async fn reopen_replica(
        mut self,
        key: PublicKeyWasm,
        ) -> Result<ReplicationHandleWasm, JsError>
    {
        self.handle
            .reopen(&key.take())
            .await.map_err(|_| JsError::new("Error re-opening replica."))?;
        Ok(self)
    }
}

/// WASM wrapper for [Replication] and [ReplicationHandle].
#[wasm_bindgen]
pub struct ReplicationWasm {
    replication: Replication<IoStream<WsStreamIo, Vec<u8>>>,
    meta: WsMeta,
    handle: ReplicationHandle,
}
#[wasm_bindgen]
impl ReplicationWasm {
    /// Create, connect, and handshake a new websocket [Replication].
    pub async fn new(url: String) -> Result<ReplicationWasm, JsError> {
        // Connect websocket.
        let (meta, ws) = WsMeta::connect(url, None).await?;
        let stream = ws.into_io();

        // Handshake
        let t = async move {
            Delay::new(Duration::from_secs(5)).await.unwrap();
            Err(JsError::new("Handshake timed out."))
        };
        let replication = async move {
            let options = Options {
                is_initiator: true,
                keepalive_ms: None,
                ..Options::default()
            };
            Replication::with_options(stream, options)
                .await.map_err(|_| JsError::new("Handshake error."))
        };
        let (replication, handle) = race(t, replication).await?;

        Ok(Self {
            replication,
            meta,
            handle,
        })
    }

    /// Get a [ReplicationHandleWasm] for this replication.
    pub fn get_handle(&self) -> ReplicationHandleWasm {
        let handle = self.handle.clone();
        ReplicationHandleWasm { handle }
    }

    /// Run [ReplicationWasm].
    pub async fn run(mut self) -> Result<(), JsError> {
        // Observe websocket events.
        let mut events = self.meta.observe(pharos::ObserveConfig::default())
            .await.map_err(
                |_| JsError::new("Error observing websocket events."))?;
        let events = async move {
            loop {
                // Unwrap event.
                let event = match events.next().await {
                    None => Err(JsError::new(
                            "Error getting websocket event.")),
                    Some(event) => Ok(event),
                }?;
                // Handle event.
                match event {
                    WsEvent::Open => Ok(()),
                    WsEvent::Error => Err(
                        JsError::new("Error websocket unspecified.")),
                    WsEvent::Closing => Err(
                        JsError::new("Websocket closing.")),
                    WsEvent::Closed(_) => Err(
                        JsError::new("Websocket closed.")),
                    WsEvent::WsErr(_) => Err(
                        JsError::new("Websocket error.")),
                }?;
            };
        };

        // Run replication
        let replication = self.replication;
        let replication = async move {
            replication.run()
                .await.map_err(|_| JsError::new("Replication error."))
        };

        race(replication, events).await?;

        Ok(())
    }
}
impl Debug for ReplicationWasm {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>)
        -> Result<(), std::fmt::Error>
    {
        write!(fmt, "ReplicationWasm")
    }
}
