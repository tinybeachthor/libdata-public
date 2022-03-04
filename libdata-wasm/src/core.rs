//! Re-export [Core] in an opinionated wrapper.

use std::panic;
use console_error_panic_hook;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsError;
use async_std::sync::{Arc, Mutex};

use libdata::{
    Core, CoreIterator,
    DiscoveryKey, PublicKey, SecretKey, discovery_key,
};
use crate::storage::{RandomAccessJs, RandomAccessWasm};
use crate::keys::{DiscoveryKeyWasm, PublicKeyWasm};

type AMC<T> = Arc<Mutex<Core<T, T, T>>>;
type CoreIter<T> = CoreIterator<T, T, T>;

/// WASM wrapper for [CoreIterator].
#[wasm_bindgen]
#[derive(Debug)]
pub struct CoreIteratorWasm {
    iter: CoreIter<RandomAccessWasm>,
    discovery_key: DiscoveryKey,
}
impl CoreIteratorWasm {
    /// Wrap [CoreIterator] and [DiscoveryKey].
    pub fn new(
        iter: CoreIter<RandomAccessWasm>,
        discovery_key: DiscoveryKey,
        ) -> Self
    {
        Self { iter, discovery_key }
    }

    /// Unwrap into [CoreIterator].
    pub fn take(self) -> (CoreIter<RandomAccessWasm>, DiscoveryKey) {
        (self.iter, self.discovery_key)
    }
}

/// WASM wrapper for [Core].
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct CoreWasm {
    core: AMC<RandomAccessWasm>,
    public_key: PublicKey,
    last_value: Option<JsValue>,
}
#[wasm_bindgen]
impl CoreWasm {
    /// Create a new [CoreWasm] from [RandomAccessJs] stores.
    pub async fn new(
        public_hex: String,
        secret_hex: Option<String>,
        data: RandomAccessJs,
        blocks: RandomAccessJs,
        merkle: RandomAccessJs,
        ) -> Result<CoreWasm, JsError>
    {
        panic::set_hook(Box::new(console_error_panic_hook::hook));

        let public = PublicKey::from_bytes(&hex::decode(&public_hex)?)?;
        let secret = match secret_hex {
            None => None,
            Some(hex) => Some(SecretKey::from_bytes(&hex::decode(&hex)?)?),
        };

        let core = Core::new(
            RandomAccessWasm::new(data),
            RandomAccessWasm::new(blocks),
            RandomAccessWasm::new(merkle),
            public, secret)
            .await.map_err(|_| JsError::new("Could not create CoreWasm."))?;

        Ok(Self {
            core: Arc::new(Mutex::new(core)),
            public_key: public,
            last_value: None,
        })
    }

    /// Returns [DiscoveryKeyWasm].
    pub fn discovery_key(&self) -> DiscoveryKeyWasm {
        let discovery_key = discovery_key(self.public_key.as_bytes());
        DiscoveryKeyWasm::new(discovery_key)
    }

    /// Returns [PublicKeyWasm].
    pub fn public_key(&self) -> PublicKeyWasm {
        PublicKeyWasm::new(self.public_key.clone())
    }

    /// Append data to the core.
    ///
    /// Because of the requirement for 'static lifetime for async wasm methods,
    /// the [CoreWasm] is threaded through.
    pub async fn append(
        self,
        data: String,
        ) -> Result<CoreWasm, JsError>
    {
        let data: Vec<u8> = data.as_bytes().to_vec();
        {
            let mut core = self.core.lock().await;
            core.append(&data, None).await
                .map_err(|_| JsError::new("Could not append data to core."))?;
        }
        Ok(self)
    }

    /// Get a value in the core at an index.
    ///
    /// Because of the requirement for 'static lifetime for async wasm methods,
    /// the [CoreWasm] is threaded through.
    /// Use [CoreWasm::read_last] to retrieve the last value got.
    pub async fn get(
        mut self,
        index: u32,
        ) -> Result<CoreWasm, JsError>
    {
        let data: Option<(Vec<u8>, _)>;
        {
            let mut core = self.core.lock().await;
            data = core.get(index).await
                .map_err(|_| JsError::new("Could not get data from core."))?;
        }

        let data = match data {
            Some((data, _)) =>
                JsValue::from_str(&String::from_utf8_lossy(&data)),
            None => JsValue::NULL,
        };
        self.last_value = Some(data);

        Ok(self)
    }
    /// Retrieve the last value got.
    pub fn read_last(
        &mut self,
        ) -> JsValue
    {
        match self.last_value.take() {
            Some(value) => value,
            None => JsValue::NULL,
        }
    }

    /// Get [CoreIteratorWasm] for the core.
    pub fn iter(&self) -> CoreIteratorWasm {
        let iter = CoreIterator::new(Arc::clone(&self.core), 0);
        CoreIteratorWasm {
            iter,
            discovery_key: discovery_key(self.public_key.as_bytes()),
        }
    }
}
impl CoreWasm {
    /// Get [&PublicKey].
    pub fn public_key_inner(&self) -> &PublicKey {
        &self.public_key
    }

    /// Unwrap into [Arc<Mutex<Core>>].
    pub fn take(self) -> AMC<RandomAccessWasm> {
        self.core
    }

    /// Get a cloned [Arc<Mutex<Core>>].
    pub fn clone_inner(&self) -> AMC<RandomAccessWasm> {
        Arc::clone(&self.core)
    }
}
