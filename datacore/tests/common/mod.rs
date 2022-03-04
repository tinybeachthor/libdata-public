#![cfg_attr(test, allow(dead_code))]

use std::path::PathBuf;

use random_access_memory::RandomAccessMemory;
use random_access_disk::RandomAccessDisk;
use datacore::Keypair;

pub fn random_access_memory() -> RandomAccessMemory {
    RandomAccessMemory::new(1024)
}
pub async fn random_access_disk(dir: PathBuf) -> RandomAccessDisk {
    RandomAccessDisk::open(dir).await.unwrap()
}

pub fn copy_keypair(keypair: &Keypair) -> Keypair {
    Keypair::from_bytes(&keypair.to_bytes()).unwrap()
}
