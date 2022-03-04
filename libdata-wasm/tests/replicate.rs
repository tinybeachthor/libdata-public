use anyhow::{Result, anyhow};
use futures_lite::future::race;
use async_std::sync::{Arc, Mutex};
use sluice::pipe::{PipeReader, PipeWriter, pipe};
use async_channel;
use std::time::Duration;
use fluvio_wasm_timer::Delay;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

use random_access_memory::RandomAccessMemory;
use libdata::{Core, PublicKey, generate_keypair};
use libdata::replication::{
    Duplex, Options, CoreReplica, Replication, ReplicationHandle};

fn random_access_memory() -> RandomAccessMemory {
    RandomAccessMemory::new(1024)
}
async fn new_core()
    -> Result<Core<RandomAccessMemory, RandomAccessMemory, RandomAccessMemory>>
{
    let keypair = generate_keypair();
    Core::new(
        random_access_memory(),
        random_access_memory(),
        random_access_memory(),
        keypair.public, Some(keypair.secret))
        .await
}
async fn new_replica(key: PublicKey)
    -> Result<Core<RandomAccessMemory, RandomAccessMemory, RandomAccessMemory>>
{
    Core::new(
        random_access_memory(),
        random_access_memory(),
        random_access_memory(),
        key, None)
        .await
}
fn create_duplex_pair_memory()
    -> (Duplex<PipeReader, PipeWriter>, Duplex<PipeReader, PipeWriter>)
{
    let (ar, bw) = pipe();
    let (br, aw) = pipe();
    (Duplex::new(ar, aw), Duplex::new(br, bw))
}
fn default_options(is_initiator: bool) -> Options {
    Options {
        is_initiator,
        keepalive_ms: None,
        ..Options::default()
    }
}
type ReplicationMemory =
    (Replication<Duplex<PipeReader, PipeWriter>>, ReplicationHandle);
async fn create_replication_pair_memory()
    -> (ReplicationMemory, ReplicationMemory)
{
    let (txa, rxa) = async_channel::bounded(1);
    let (txb, rxb) = async_channel::bounded(1);

    let (a_stream, b_stream) = create_duplex_pair_memory();
    spawn_local(async move {
        let t = async move {
            Delay::new(Duration::from_secs(1)).await.unwrap();
            Err(anyhow!("timed out"))
        };
        let replication = Replication::with_options(
            a_stream, default_options(false));
        let replication = race(replication, t).await.unwrap();
        txa.send(replication).await.unwrap();
    });
    spawn_local(async move {
        let t = async move {
            Delay::new(Duration::from_secs(1)).await.unwrap();
            Err(anyhow!("timed out"))
        };
        let replication = Replication::with_options(
            b_stream, default_options(true));
        let replication = race(replication, t).await.unwrap();
        txb.send(replication).await.unwrap();
    });

    let a = rxa.recv().await.unwrap();
    let b = rxb.recv().await.unwrap();
    (a, b)
}

#[wasm_bindgen_test]
async fn replication_core_replica() {
    let mut a = new_core().await.unwrap();
    let public = a.public_key().clone();
    let b = new_replica(public.clone()).await.unwrap();

    let data = b"hello world";
    a.append(data, None).await.unwrap();

    let a_replica = Box::new(CoreReplica::new(Arc::new(Mutex::new(a))));
    let b = Arc::new(Mutex::new(b));
    let b_replica = Box::new(CoreReplica::new(Arc::clone(&b)));

    let (txa, rxa) = async_channel::bounded(1);
    let (txb, rxb) = async_channel::bounded(1);
    let ((a_replication, mut a_handle), (b_replication, mut b_handle)) =
        create_replication_pair_memory().await;
    spawn_local(async move {
        let t = async move {
            Delay::new(Duration::from_secs(1)).await.unwrap();
        };
        race(t, async move {
            a_handle.open(&public, a_replica).await.unwrap();
            let _ = a_replication.run().await;
        }).await;
        txa.send(()).await.unwrap();
    });
    spawn_local(async move {
        let t = async move {
            Delay::new(Duration::from_secs(1)).await.unwrap();
        };
        race(t, async move {
            b_handle.open(&public, b_replica).await.unwrap();
            let _ = b_replication.run().await;
        }).await;
        txb.send(()).await.unwrap();
    });
    let _ = rxa.recv().await.unwrap();
    let _ = rxb.recv().await.unwrap();

    let mut b = b.lock().await;
    assert_eq!(b.get(0).await.unwrap().unwrap().0, data);
}
