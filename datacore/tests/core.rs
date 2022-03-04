mod common;
use common::{random_access_memory, random_access_disk, copy_keypair};

use async_std::test;
use tempfile;

use datacore::{
    Merkle, NodeTrait, Hash, BlockSignature, Core,
    generate_keypair, sign,
};

#[test]
pub async fn core_init() {
    let keypair = generate_keypair();
    let core = Core::new(
        random_access_memory(),
        random_access_memory(),
        random_access_memory(),
        keypair.public, Some(keypair.secret))
        .await.unwrap();

    assert_eq!(core.len(), 0);
}

#[test]
pub async fn core_append() {
    let keypair = generate_keypair();
    let mut core = Core::new(
        random_access_memory(),
        random_access_memory(),
        random_access_memory(),
        keypair.public, Some(keypair.secret))
        .await.unwrap();

    core.append(br#"{"hello":"world"}"#, None).await.unwrap();
    core.append(br#"{"hello":"mundo"}"#, None).await.unwrap();
    core.append(br#"{"hello":"welt"}"#, None).await.unwrap();

    assert_eq!(core.len(), 3);
    assert_eq!(
        core.get(0).await.unwrap().map(first),
        Some(br#"{"hello":"world"}"#.to_vec()));
    assert_eq!(
        core.get(1).await.unwrap().map(first),
        Some(br#"{"hello":"mundo"}"#.to_vec()));
    assert_eq!(
        core.get(2).await.unwrap().map(first),
        Some(br#"{"hello":"welt"}"#.to_vec()));
}

#[test]
pub async fn core_signatures() {
    let keypair = generate_keypair();
    let keypair2 = copy_keypair(&keypair);
    let mut core = Core::new(
        random_access_memory(),
        random_access_memory(),
        random_access_memory(),
        keypair.public, Some(keypair.secret))
        .await.unwrap();

    let data1 = b"hello world";
    let data2 = b"this is datacore";

    core.append(data1, None).await.unwrap();
    core.append(data2, None).await.unwrap();

    let mut merkle = Merkle::new();
    merkle.next(Hash::from_leaf(data1), data1.len() as u64);
    let signature1 = BlockSignature::new(
        sign(&keypair2.public, &keypair2.secret, &Hash::from_leaf(data1)),
        sign(&keypair2.public, &keypair2.secret, &hash_tree(&merkle)));
    merkle.next(Hash::from_leaf(data2), data2.len() as u64);
    let signature2 = BlockSignature::new(
        sign(&keypair2.public, &keypair2.secret, &Hash::from_leaf(data2)),
        sign(&keypair2.public, &keypair2.secret, &hash_tree(&merkle)));

    assert_eq!(core.len(), 2);
    assert_eq!(
        core.get(0).await.unwrap(),
        Some((data1.to_vec(), signature1)));
    assert_eq!(
        core.get(1).await.unwrap(),
        Some((data2.to_vec(), signature2)));
}

#[test]
pub async fn core_get_head() {
    let keypair = generate_keypair();
    let mut core = Core::new(
        random_access_memory(),
        random_access_memory(),
        random_access_memory(),
        keypair.public, Some(keypair.secret))
        .await.unwrap();

    assert_eq!(core.len(), 0);
    assert_eq!(core.head().await.unwrap(), None);

    core.append(br#"{"hello":"world"}"#, None).await.unwrap();
    core.append(br#"{"hello":"mundo"}"#, None).await.unwrap();
    core.append(br#"{"hello":"welt"}"#, None).await.unwrap();

    assert_eq!(core.len(), 3);
    assert_eq!(
        core.get(1).await.unwrap().map(first),
        Some(br#"{"hello":"mundo"}"#.to_vec()));
    assert_eq!(
        core.get(2).await.unwrap().map(first),
        Some(br#"{"hello":"welt"}"#.to_vec()));
    assert_eq!(
        core.head().await.unwrap().map(first),
        Some(br#"{"hello":"welt"}"#.to_vec()));
}

#[test]
pub async fn core_append_no_secret_key() {
    let keypair = generate_keypair();
    let mut core = Core::new(
        random_access_memory(),
        random_access_memory(),
        random_access_memory(),
        keypair.public, None)
        .await.unwrap();

    assert!(core.append(b"hello", None).await.is_err());
    assert_eq!(core.len(), 0);
}

#[test]
pub async fn core_disk_append() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let keypair = generate_keypair();
    let mut core = Core::new(
        random_access_disk(dir.to_path_buf().join("d")).await,
        random_access_disk(dir.to_path_buf().join("b")).await,
        random_access_disk(dir.to_path_buf().join("s")).await,
        keypair.public, Some(keypair.secret))
        .await.unwrap();

    core.append(b"hello world", None).await.unwrap();
    core.append(b"this is datacore", None).await.unwrap();

    assert_eq!(core.len(), 2);
    assert_eq!(
        core.get(0).await.unwrap().map(first),
        Some(b"hello world".to_vec()));
    assert_eq!(
        core.get(1).await.unwrap().map(first),
        Some(b"this is datacore".to_vec()));
}

#[test]
pub async fn core_disk_persists() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let keypair = generate_keypair();
    let keypair2 = copy_keypair(&keypair);
    let mut core = Core::new(
        random_access_disk(dir.to_path_buf().join("d")).await,
        random_access_disk(dir.to_path_buf().join("b")).await,
        random_access_disk(dir.to_path_buf().join("s")).await,
        keypair.public, Some(keypair.secret))
        .await.unwrap();

    core.append(b"hello world", None).await.unwrap();
    core.append(b"this is datacore", None).await.unwrap();

    let mut core = Core::new(
        random_access_disk(dir.to_path_buf().join("d")).await,
        random_access_disk(dir.to_path_buf().join("b")).await,
        random_access_disk(dir.to_path_buf().join("s")).await,
        keypair2.public, Some(keypair2.secret))
        .await.unwrap();

    assert_eq!(core.len(), 2);
    assert_eq!(
        core.get(0).await.unwrap().map(first),
        Some(b"hello world".to_vec()));
    assert_eq!(
        core.get(1).await.unwrap().map(first),
        Some(b"this is datacore".to_vec()));
}

fn first<A, B>(t: (A, B)) -> A {
    t.0
}
fn hash_tree(merkle: &Merkle) -> Hash {
    let roots = merkle.roots();
    let hashes = roots.iter()
        .map(|root| root.hash())
        .collect::<Vec<&Hash>>();
    let lengths = roots.iter()
        .map(|root| root.len())
        .collect::<Vec<u64>>();
    Hash::from_roots(&hashes, &lengths)
}
