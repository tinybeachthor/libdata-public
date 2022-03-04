use anyhow::Result;
use std::time::Duration;
use futures_lite::future::zip;
use async_std::{test, task};
use async_std::sync::{Arc, Mutex};
use sluice::pipe::{PipeReader, PipeWriter, pipe};

use random_access_memory::RandomAccessMemory;
use libdata::{generate_keypair, PublicKey, Core};
use libdata::replication::{
    CoreReplica, Duplex, Replication, Options, ReplicationHandle,
};

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

type ReplicationMemory =
    (Replication<Duplex<PipeReader, PipeWriter>>, ReplicationHandle);

fn create_duplex_pair_memory()
    -> (Duplex<PipeReader, PipeWriter>, Duplex<PipeReader, PipeWriter>)
{
    let (ar, bw) = pipe();
    let (br, aw) = pipe();
    (Duplex::new(ar, aw), Duplex::new(br, bw))
}
async fn create_replication_pair_memory()
    -> (ReplicationMemory, ReplicationMemory)
{
    const KEEPALIVE_MS: u64 = 500;

    let (a_stream, b_stream) = create_duplex_pair_memory();
    zip(
        task::spawn(async move {
            Replication::with_options(a_stream, Options {
                is_initiator: false,
                keepalive_ms: Some(KEEPALIVE_MS),
                ..Options::default()
            }).await.unwrap()
        }),
        task::spawn(async move {
            Replication::with_options(b_stream, Options {
                is_initiator: true,
                keepalive_ms: Some(KEEPALIVE_MS),
                ..Options::default()
            }).await.unwrap()
        })
    ).await
}

#[test]
async fn replication_core_replica() -> Result<()>
{
    let mut a = new_core().await?;
    let public = a.public_key().clone();
    let b = new_replica(public.clone()).await?;

    let data = b"hello world";
    a.append(data, None).await?;

    let a_replica = Box::new(CoreReplica::new(Arc::new(Mutex::new(a))));
    let b = Arc::new(Mutex::new(b));
    let b_replica = Box::new(CoreReplica::new(Arc::clone(&b)));

    let ((a_replication, mut a_handle),
         (b_replication, mut b_handle)) =
        create_replication_pair_memory().await;
    zip(
        task::spawn(async move {
            a_handle.open(&public, a_replica).await.unwrap();
            a_replication.run().await.unwrap();
        }),
        task::spawn(async move {
            b_handle.open(&public, b_replica).await.unwrap();
            b_replication.run().await.unwrap();
        })
    ).await;

    let mut b = b.lock().await;
    assert_eq!(b.get(0).await?.unwrap().0, data);
    Ok(())
}
#[test]
async fn replication_core_replica_async_open() -> Result<()>
{
    let mut a = new_core().await?;
    let public = a.public_key().clone();
    let b = new_replica(public.clone()).await?;

    let data = b"hello world";
    a.append(data, None).await?;

    let a_replica = Box::new(CoreReplica::new(Arc::new(Mutex::new(a))));
    let b = Arc::new(Mutex::new(b));
    let b_replica = Box::new(CoreReplica::new(Arc::clone(&b)));

    let ((a_replication, mut a_handle),
         (b_replication, mut b_handle)) =
        create_replication_pair_memory().await;
    zip(
        zip(
            task::spawn(async move {
                a_replication.run().await.unwrap();
            }),
            task::spawn(async move {
                b_replication.run().await.unwrap();
            })
        ),
        zip(
            task::spawn(async move {
                a_handle.open(&public, a_replica).await.unwrap();
            }),
            task::spawn(async move {
                b_handle.open(&public, b_replica).await.unwrap();
            })
        ),
    ).await;

    let mut b = b.lock().await;
    assert_eq!(b.get(0).await?.unwrap().0, data);
    Ok(())
}

#[test]
async fn replication_core_replica_multiple_blocks() -> Result<()>
{
    let mut a = new_core().await?;
    let public = a.public_key().clone();
    let b = new_replica(public.clone()).await?;

    let data = b"hello world";
    for &d in data.into_iter() {
        a.append(&[d], None).await?;
    }

    let a_replica = Box::new(CoreReplica::new(Arc::new(Mutex::new(a))));
    let b = Arc::new(Mutex::new(b));
    let b_replica = Box::new(CoreReplica::new(Arc::clone(&b)));

    let ((a_replication, mut a_handle),
         (b_replication, mut b_handle)) =
        create_replication_pair_memory().await;
    let (a_result, b_result) = zip(
        task::spawn(async move {
            a_handle.open(&public, a_replica).await.unwrap();
            a_replication.run().await
        }),
        task::spawn(async move {
            b_handle.open(&public, b_replica).await.unwrap();
            b_replication.run().await
        })
    ).await;
    a_result?;
    b_result?;

    let mut b = b.lock().await;
    for (i, &d) in data.into_iter().enumerate() {
        assert_eq!(b.get(i as u32).await?.unwrap().0[0], d);
    }
    Ok(())
}

#[test]
async fn replication_core_replica_multiple_blocks_live() -> Result<()>
{
    let a = new_core().await?;
    let public = a.public_key().clone();
    let b = new_replica(public.clone()).await?;

    let data = b"hello world";

    let a = Arc::new(Mutex::new(a));
    let a_replica = Box::new(CoreReplica::new(Arc::clone(&a)));
    let b = Arc::new(Mutex::new(b));
    let b_replica = Box::new(CoreReplica::new(Arc::clone(&b)));

    let ((a_replication, mut a_handle),
         (b_replication, mut b_handle)) =
        create_replication_pair_memory().await;
    zip(
        zip(
            task::spawn(async move {
                a_replication.run().await.unwrap();
            }),
            task::spawn(async move {
                b_replication.run().await.unwrap();
            })
        ),
        zip(
            task::spawn(async move {
                a_handle.open(&public, a_replica).await.unwrap();
                for &d in data.into_iter() {
                    let mut a = a.lock().await;
                    a.append(&[d], None).await.unwrap();
                    a_handle.reopen(&public).await.unwrap();
                    task::sleep(Duration::from_millis(10)).await;
                }
            }),
            task::spawn(async move {
                b_handle.open(&public, b_replica).await.unwrap();
            })
        ),
    ).await;

    let mut b = b.lock().await;
    for (i, &d) in data.into_iter().enumerate() {
        assert_eq!(b.get(i as u32).await?.unwrap().0[0], d);
    }
    Ok(())
}

#[test]
async fn replication_core_replica_of_replica() -> Result<()>
{
    let mut a = new_core().await?;
    let public = a.public_key().clone();
    let b = new_replica(public.clone()).await?;
    let c = new_replica(public.clone()).await?;

    let data = b"hello world";
    a.append(data, None).await?;

    let a_replica = Box::new(CoreReplica::new(Arc::new(Mutex::new(a))));
    let b = Arc::new(Mutex::new(b));
    let b_replica = Box::new(CoreReplica::new(Arc::clone(&b)));
    let b2_replica = Box::new(CoreReplica::new(Arc::clone(&b)));
    let c = Arc::new(Mutex::new(c));
    let c_replica = Box::new(CoreReplica::new(Arc::clone(&c)));

    let ((a_replication, mut a_handle),
         (b_replication, mut b_handle)) =
        create_replication_pair_memory().await;
    zip(
        task::spawn(async move {
            a_handle.open(&public, a_replica).await.unwrap();
            a_replication.run().await.unwrap();
        }),
        task::spawn(async move {
            b_handle.open(&public, b_replica).await.unwrap();
            b_replication.run().await.unwrap();
        })
    ).await;

    let ((b2_replication, mut b2_handle),
         (c_replication, mut c_handle)) =
        create_replication_pair_memory().await;
    zip(
        task::spawn(async move {
            b2_handle.open(&public, b2_replica).await.unwrap();
            b2_replication.run().await.unwrap();
        }),
        task::spawn(async move {
            c_handle.open(&public, c_replica).await.unwrap();
            c_replication.run().await.unwrap();
        })
    ).await;

    let mut c = c.lock().await;
    assert_eq!(c.get(0).await?.unwrap().0, data);
    Ok(())
}
