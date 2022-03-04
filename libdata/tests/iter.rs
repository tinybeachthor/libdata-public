use anyhow::Result;
use futures_lite::stream::StreamExt;
use async_std::sync::{Arc, Mutex};
use async_std::test;

use random_access_memory::RandomAccessMemory;
use libdata::{generate_keypair, Core, CoreIterator};

fn random_access_memory() -> RandomAccessMemory {
    RandomAccessMemory::new(1024)
}

#[test]
async fn iter_simple() -> Result<()>
{
    let keypair = generate_keypair();
    let mut core = Core::new(
        random_access_memory(),
        random_access_memory(),
        random_access_memory(),
        keypair.public, Some(keypair.secret))
        .await.unwrap();

    let data = vec![1, 2, 3];
    for d in data {
        core.append(&[d], None).await.unwrap();
    }

    let mut iter = CoreIterator::new(Arc::new(Mutex::new(core)), 0);
    assert_eq!(iter.next().await.unwrap(), (0, vec![1]));
    assert_eq!(iter.next().await.unwrap(), (1, vec![2]));
    assert_eq!(iter.next().await.unwrap(), (2, vec![3]));
    assert_eq!(iter.next().await, None);
    Ok(())
}

#[test]
async fn iter_offset() -> Result<()>
{
    let keypair = generate_keypair();
    let mut core = Core::new(
        random_access_memory(),
        random_access_memory(),
        random_access_memory(),
        keypair.public, Some(keypair.secret))
        .await.unwrap();

    let data = vec![1, 2, 3];
    for d in data {
        core.append(&[d], None).await.unwrap();
    }

    let mut iter = CoreIterator::new(Arc::new(Mutex::new(core)), 1);
    assert_eq!(iter.next().await.unwrap(), (1, vec![2]));
    assert_eq!(iter.next().await.unwrap(), (2, vec![3]));
    assert_eq!(iter.next().await, None);
    Ok(())
}

#[test]
async fn iter_out_of_bounds() -> Result<()>
{
    let keypair = generate_keypair();
    let mut core = Core::new(
        random_access_memory(),
        random_access_memory(),
        random_access_memory(),
        keypair.public, Some(keypair.secret))
        .await.unwrap();

    let data = vec![1, 2, 3];
    for d in data {
        core.append(&[d], None).await.unwrap();
    }

    let mut iter = CoreIterator::new(Arc::new(Mutex::new(core)), 100);
    assert_eq!(iter.next().await, None);
    Ok(())
}
