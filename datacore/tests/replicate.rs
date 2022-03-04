mod common;
use common::{random_access_disk, copy_keypair};

use std::path::Path;
use std::fs::File;
use std::io::Read;
use async_std::test;
use tempfile;

use datacore::{
    Core, Merkle, Signature, BlockSignature, Hash, NodeTrait,
    generate_keypair, sign, verify, SIGNATURE_LENGTH,
};

fn read_bytes(dir: &Path, s: &str) -> Vec<u8> {
    let mut f = File::open(dir.join(s)).unwrap();
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).unwrap();
    bytes
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

#[test]
pub async fn replicate_manual() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();
    let keypair = generate_keypair();
    let keypair2 = copy_keypair(&keypair);
    let keypair3 = copy_keypair(&keypair);

    let mut core = Core::new(
        random_access_disk(dir.to_path_buf().join("data")).await,
        random_access_disk(dir.to_path_buf().join("blocks")).await,
        random_access_disk(dir.to_path_buf().join("merkle")).await,
        keypair.public, Some(keypair.secret))
        .await.unwrap();
    let mut replica = Core::new(
        random_access_disk(dir2.to_path_buf().join("data")).await,
        random_access_disk(dir2.to_path_buf().join("blocks")).await,
        random_access_disk(dir2.to_path_buf().join("merkle")).await,
        keypair2.public, Some(keypair2.secret))
        .await.unwrap();

    let data1 = b"hello world";
    let data2 = b"this is datacore";

    core.append(data1, None).await.unwrap();
    core.append(data2, None).await.unwrap();
    assert_eq!(core.len(), 2);

    let mut merkle = Merkle::new();
    let data_hash = Hash::from_leaf(data1);
    let data_sign = sign(&keypair3.public, &keypair3.secret, &data_hash);
    merkle.next(data_hash.clone(), data1.len() as u64);
    verify(&keypair3.public, &data_hash, &data_sign).unwrap();
    let tree_hash = hash_tree(&merkle);
    let tree_sign = sign(&keypair3.public, &keypair3.secret, &tree_hash);
    verify(&keypair3.public, &tree_hash, &tree_sign).unwrap();
    let signature = BlockSignature::new(data_sign, tree_sign);
    replica.append(data1, Some(signature)).await.unwrap();
    let data_hash = Hash::from_leaf(data2);
    merkle.next(data_hash.clone(), data2.len() as u64);
    let signature = BlockSignature::new(
        sign(&keypair3.public, &keypair3.secret, &data_hash),
        sign(&keypair3.public, &keypair3.secret, &hash_tree(&merkle)));
    replica.append(data2, Some(signature)).await.unwrap();
    assert_eq!(replica.len(), 2);

    assert_eq!(
        read_bytes(&dir2, "data"), read_bytes(&dir, "data"));
    assert_eq!(
        read_bytes(&dir2, "blocks"), read_bytes(&dir, "blocks"));
    assert_eq!(
        read_bytes(&dir2, "merkle"), read_bytes(&dir, "merkle"));
}

#[test]
pub async fn replicate_manual_no_secret_key() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();
    let keypair = generate_keypair();
    let keypair2 = copy_keypair(&keypair);
    let keypair3 = copy_keypair(&keypair);

    let mut core = Core::new(
        random_access_disk(dir.to_path_buf().join("data")).await,
        random_access_disk(dir.to_path_buf().join("blocks")).await,
        random_access_disk(dir.to_path_buf().join("merkle")).await,
        keypair.public, Some(keypair.secret))
        .await.unwrap();
    let mut replica = Core::new(
        random_access_disk(dir2.to_path_buf().join("data")).await,
        random_access_disk(dir2.to_path_buf().join("blocks")).await,
        random_access_disk(dir2.to_path_buf().join("merkle")).await,
        keypair2.public, Some(keypair2.secret))
        .await.unwrap();

    let data1 = b"hello world";
    let data2 = b"this is datacore";

    core.append(data1, None).await.unwrap();
    core.append(data2, None).await.unwrap();
    assert_eq!(core.len(), 2);

    let mut merkle = Merkle::new();
    let data_hash = Hash::from_leaf(data1);
    merkle.next(data_hash.clone(), data1.len() as u64);
    let signature = BlockSignature::new(
        sign(&keypair3.public, &keypair3.secret, &data_hash),
        sign(&keypair3.public, &keypair3.secret, &hash_tree(&merkle)));
    replica.append(data1, Some(signature)).await.unwrap();
    let data_hash = Hash::from_leaf(data2);
    merkle.next(data_hash.clone(), data2.len() as u64);
    let signature = BlockSignature::new(
        sign(&keypair3.public, &keypair3.secret, &data_hash),
        sign(&keypair3.public, &keypair3.secret, &hash_tree(&merkle)));
    replica.append(data2, Some(signature)).await.unwrap();
    assert_eq!(replica.len(), 2);

    assert_eq!(
        read_bytes(&dir2, "data"), read_bytes(&dir, "data"));
    assert_eq!(
        read_bytes(&dir2, "blocks"), read_bytes(&dir, "blocks"));
    assert_eq!(
        read_bytes(&dir2, "merkle"), read_bytes(&dir, "merkle"));
}

#[test]
pub async fn replicate_signatures_no_secret_key() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();
    let keypair = generate_keypair();
    let keypair2 = copy_keypair(&keypair);

    let mut core = Core::new(
        random_access_disk(dir.to_path_buf().join("data")).await,
        random_access_disk(dir.to_path_buf().join("blocks")).await,
        random_access_disk(dir.to_path_buf().join("merkle")).await,
        keypair.public, Some(keypair.secret))
        .await.unwrap();
    let mut replica = Core::new(
        random_access_disk(dir2.to_path_buf().join("data")).await,
        random_access_disk(dir2.to_path_buf().join("blocks")).await,
        random_access_disk(dir2.to_path_buf().join("merkle")).await,
        keypair2.public, Some(keypair2.secret))
        .await.unwrap();

    let data1 = b"hello world";
    let data2 = b"this is datacore";

    core.append(data1, None).await.unwrap();
    core.append(data2, None).await.unwrap();
    assert_eq!(core.len(), 2);

    let (data1, signature) = core.get(0).await.unwrap().unwrap();
    replica.append(&data1, Some(signature)).await.unwrap();
    let (data2, signature) = core.get(1).await.unwrap().unwrap();
    replica.append(&data2, Some(signature)).await.unwrap();
    assert_eq!(replica.len(), 2);

    assert_eq!(
        read_bytes(&dir2, "data"), read_bytes(&dir, "data"));
    assert_eq!(
        read_bytes(&dir2, "blocks"), read_bytes(&dir, "blocks"));
    assert_eq!(
        read_bytes(&dir2, "merkle"), read_bytes(&dir, "merkle"));
}

#[test]
pub async fn replicate_then_append() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();
    let keypair = generate_keypair();
    let keypair2 = copy_keypair(&keypair);

    let mut core = Core::new(
        random_access_disk(dir.to_path_buf().join("data")).await,
        random_access_disk(dir.to_path_buf().join("blocks")).await,
        random_access_disk(dir.to_path_buf().join("merkle")).await,
        keypair.public, Some(keypair.secret))
        .await.unwrap();
    let mut replica = Core::new(
        random_access_disk(dir2.to_path_buf().join("data")).await,
        random_access_disk(dir2.to_path_buf().join("blocks")).await,
        random_access_disk(dir2.to_path_buf().join("merkle")).await,
        keypair2.public, Some(keypair2.secret))
        .await.unwrap();

    let data1 = b"hello world";
    let data2 = b"this is datacore";
    let data3 = b"THIS WILL NOT BE REPLICATED";

    core.append(data1, None).await.unwrap();
    core.append(data2, None).await.unwrap();
    core.append(data3, None).await.unwrap();
    assert_eq!(core.len(), 3);

    let (data1, signature) = core.get(0).await.unwrap().unwrap();
    replica.append(&data1, Some(signature)).await.unwrap();
    let (data2, signature) = core.get(1).await.unwrap().unwrap();
    replica.append(&data2, Some(signature)).await.unwrap();
    assert_eq!(replica.len(), 2);

    replica.append(data3, None).await.unwrap();
    assert_eq!(replica.len(), 3);

    assert_eq!(
        read_bytes(&dir2, "data"), read_bytes(&dir, "data"));
    assert_eq!(
        read_bytes(&dir2, "blocks"), read_bytes(&dir, "blocks"));
    assert_eq!(
        read_bytes(&dir2, "merkle"), read_bytes(&dir, "merkle"));
}

#[test]
pub async fn replicate_fail_verify_then_append() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();
    let keypair = generate_keypair();
    let keypair2 = copy_keypair(&keypair);

    let mut core = Core::new(
        random_access_disk(dir.to_path_buf().join("data")).await,
        random_access_disk(dir.to_path_buf().join("blocks")).await,
        random_access_disk(dir.to_path_buf().join("merkle")).await,
        keypair.public, Some(keypair.secret))
        .await.unwrap();
    let mut replica = Core::new(
        random_access_disk(dir2.to_path_buf().join("data")).await,
        random_access_disk(dir2.to_path_buf().join("blocks")).await,
        random_access_disk(dir2.to_path_buf().join("merkle")).await,
        keypair2.public, Some(keypair2.secret))
        .await.unwrap();

    let data1 = b"hello world";
    let data2 = b"this is datacore";
    let data3 = b"THIS WILL NOT BE REPLICATED";

    core.append(data1, None).await.unwrap();
    core.append(data2, None).await.unwrap();
    core.append(data3, None).await.unwrap();
    assert_eq!(core.len(), 3);

    let (data1, signature) = core.get(0).await.unwrap().unwrap();
    replica.append(&data1, Some(signature)).await.unwrap();
    let (data2, signature) = core.get(1).await.unwrap().unwrap();
    let invalid_signature_1 = BlockSignature::new(
        signature.data(),
        Signature::from_bytes(&[0u8; SIGNATURE_LENGTH]).unwrap());
    let invalid_signature_2 = BlockSignature::new(
        Signature::from_bytes(&[0u8; SIGNATURE_LENGTH]).unwrap(),
        Signature::from_bytes(&[0u8; SIGNATURE_LENGTH]).unwrap());
    let invalid_signature_3 = BlockSignature::new(
        Signature::from_bytes(&[0u8; SIGNATURE_LENGTH]).unwrap(),
        signature.tree());
    assert!(replica.append(&data2, Some(invalid_signature_1)).await.is_err());
    assert!(replica.append(&data2, Some(invalid_signature_2)).await.is_err());
    assert!(replica.append(&data2, Some(invalid_signature_3)).await.is_err());
    replica.append(&data2, Some(signature)).await.unwrap();
    assert_eq!(replica.len(), 2);

    replica.append(data3, None).await.unwrap();
    assert_eq!(replica.len(), 3);

    assert_eq!(
        read_bytes(&dir2, "data"), read_bytes(&dir, "data"));
    assert_eq!(
        read_bytes(&dir2, "blocks"), read_bytes(&dir, "blocks"));
    assert_eq!(
        read_bytes(&dir2, "merkle"), read_bytes(&dir, "merkle"));
}
