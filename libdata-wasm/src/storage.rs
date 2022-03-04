//! Provide a way to access JS RandomAccess instance from WASM.

use anyhow::anyhow;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use js_sys::Uint8Array;
use async_std::sync::{Arc, Mutex};
use async_channel;

use random_access_storage::RandomAccess;

/// [RandomAccessWasm] creates a [RandomAccess] interface from
/// a JS object with `read_js` and `write_js` methods.
#[derive(Debug)]
pub struct RandomAccessWasm (Arc<Mutex<RandomAccessJs>>);
impl RandomAccessWasm {
    /// Create a new [RandomAccessWasm].
    pub fn new (ram: RandomAccessJs) -> RandomAccessWasm {
        RandomAccessWasm (Arc::new(Mutex::new(ram)))
    }
}
#[async_trait::async_trait]
impl RandomAccess for RandomAccessWasm {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    /// Write bytes at an offset to the backend.
    async fn write(
        &mut self,
        offset: u64,
        data: &[u8],
        ) -> Result<(), Self::Error>
    {
        let data = Vec::from(data);
        let this = Arc::clone(&self.0);
        let (tx, rx) = async_channel::bounded(1);

        spawn_local(async move {
            let data = Uint8Array::from(data.as_slice());
            let ram = this.lock().await;
            let result = ram.write_js(offset, data.into()).await
                .map_err(|_| anyhow!("Error calling write_js.").into());
            tx.send(result).await.unwrap();
        });

        rx.recv().await?
    }

    /// Read a sequence of bytes at an offset from the backend.
    async fn read(
        &mut self,
        offset: u64,
        length: u64,
        ) -> Result<Vec<u8>, Self::Error>
    {
        let this = Arc::clone(&self.0);
        let (tx, rx) = async_channel::bounded(1);

        spawn_local(async move {
            let ram = this.lock().await;
            let result = ram.read_js(offset, length).await
                .map(|js| Uint8Array::new(&js).to_vec())
                .map_err(|_| anyhow!("Error calling read_js.").into());
            tx.send(result).await.unwrap();
        });

        rx.recv().await?
    }
}

#[wasm_bindgen]
extern "C" {
    #[derive(Debug)]
    pub type RandomAccessJs;

    #[allow(unsafe_code)]
    #[wasm_bindgen(structural, method, catch)]
    async fn read_js(this: &RandomAccessJs, offset: u64, length: u64)
        -> Result<JsValue, JsValue>;

    #[allow(unsafe_code)]
    #[wasm_bindgen(structural, method, catch)]
    async fn write_js(this: &RandomAccessJs, offset: u64, data: JsValue)
        -> Result<(), JsValue>;
}
#[allow(unsafe_code)]
unsafe impl Send for RandomAccessJs {}
#[allow(unsafe_code)]
unsafe impl Sync for RandomAccessJs {}
