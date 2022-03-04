//! Re-export [DiscoveryKey] and [PublicKey] in an opinionated wrapper.

use wasm_bindgen::prelude::*;
use hex;

use libdata::{DiscoveryKey, PublicKey, discovery_key};

/// WASM wrapper for [PublicKey].
#[wasm_bindgen]
#[derive(Debug)]
pub struct PublicKeyWasm {
    key: PublicKey,
}
impl PublicKeyWasm {
    /// Wrap.
    pub fn new(key: PublicKey) -> Self {
        Self { key }
    }
    /// Unwrap.
    pub fn take(self) -> PublicKey {
        self.key
    }
}

/// WASM wrapper for [DiscoveryKey].
#[wasm_bindgen]
#[derive(Debug)]
pub struct DiscoveryKeyWasm {
    key: DiscoveryKey,
}
#[wasm_bindgen]
impl DiscoveryKeyWasm {
    /// Create from a hex [String].
    pub fn from_hex(hex: String) -> Result<DiscoveryKeyWasm, JsError> {
        let bytes = hex::decode(&hex)?;
        let key = bytes.try_into()
            .map_err(|_| JsError::new("Wrong length for DiscoveryKey."))?;
        Ok(DiscoveryKeyWasm { key })
    }
    /// Create from a public key hex [String].
    pub fn from_public_key_hex(hex: String)
        -> Result<DiscoveryKeyWasm, JsError>
    {
        let bytes = hex::decode(&hex)?;
        let public = PublicKey::from_bytes(&bytes)?;
        let key = discovery_key(public.as_bytes());
        Ok(DiscoveryKeyWasm { key })
    }
    /// Returns a hex [String].
    pub fn as_hex(&self) -> String {
        hex::encode(&self.key)
    }
}
impl DiscoveryKeyWasm {
    /// Wrap.
    pub fn new(key: DiscoveryKey) -> Self {
        Self { key }
    }
    /// Unwrap.
    pub fn take(self) -> DiscoveryKey {
        self.key
    }
}
