//! [MultiCoreWasm] groups related cores:
//! - keeps 1 writable (local) [Core].
//! - multiple read-only (remote) [Core]s.
//!
//! Useful for distributed CRDTs.

use wasm_bindgen::prelude::*;
use hex;

use libdata::{Cores, CoreIterator, PublicKey, discovery_key};
use libdata::replication::{CoreReplica, ReplicaTrait};
use crate::storage::RandomAccessWasm;
use crate::core::{CoreWasm, CoreIteratorWasm};
use crate::websocket::ReplicasWasm;

type HomogenousCores<T> = Cores<T, T, T>;

/// WASM wrapper for a vector of [CoreIteratorWasm]s.
#[wasm_bindgen]
#[derive(Debug)]
pub struct MultiCoreIteratorsWasm {
    iters: Vec<CoreIteratorWasm>,
}
impl MultiCoreIteratorsWasm {
    /// Unwrap.
    pub fn take(self) -> Vec<CoreIteratorWasm> {
        self.iters
    }
}

/// WASM wrapper for: one writable, multiple readable [Core]s.
#[wasm_bindgen]
#[derive(Debug)]
pub struct MultiCoreWasm {
    local: CoreWasm,
    cores: HomogenousCores<RandomAccessWasm>,
}
#[wasm_bindgen]
impl MultiCoreWasm {
    /// Create a new [MultiCoreWasm].
    pub fn new(local: CoreWasm) -> Self {
        Self {
            local,
            cores: Cores::new(),
        }
    }

    /// Access the local writable [CoreWasm].
    pub fn local(&self) -> CoreWasm {
        self.local.clone()
    }

    /// Insert a new [CoreWasm].
    pub fn insert(&mut self, core: CoreWasm) {
        let public_key = core.public_key_inner().clone();
        let core = core.take();
        self.cores.put(&public_key, core);
    }

    /// Returns `true` if contains a [Core] under the given public key.
    pub fn contains(&self, hex: &str) -> bool {
        let bytes = hex::decode(hex).unwrap();
        let public_key = PublicKey::from_bytes(&bytes).unwrap();

        *self.local.public_key_inner() == public_key
            || self.cores.get_by_public(&public_key).is_some()
    }

    /// Returns [MultiCoreIteratorsWasm].
    pub fn iters(&self) -> MultiCoreIteratorsWasm
    {
        let mut iters = Vec::with_capacity(self.cores.len() + 1);

        iters.push(self.local.iter());

        for (public_key, core) in self.cores.entries() {
            let iter = CoreIteratorWasm::new(
                CoreIterator::new(core, 0),
                discovery_key(public_key.as_bytes()),
            );
            iters.push(iter);
        }

        MultiCoreIteratorsWasm { iters }
    }

    /// Returns [ReplicasWasm].
    pub fn replicas(&self) -> ReplicasWasm
    {
        let mut replicas = Vec::with_capacity(self.cores.len() + 1);

        let public_key = self.local.public_key_inner().clone();
        let core = self.local.clone_inner();
        let replica = Box::new(CoreReplica::new(core));
        replicas.push((public_key, replica as Box<dyn ReplicaTrait + Send>));

        for (public_key, core) in self.cores.entries() {
            let replica = Box::new(CoreReplica::new(core));
            replicas.push(
                (public_key, replica as Box<dyn ReplicaTrait + Send>));
        }

        ReplicasWasm::new(replicas)
    }
}
