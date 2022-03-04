use anyhow::{Result, ensure};
use std::mem::size_of;
use std::ops::Deref;
use byteorder::{LittleEndian, WriteBytesExt};
use blake3::Hasher;

const HASH_LENGTH: usize = 32;

// https://en.wikipedia.org/wiki/Merkle_tree#Second_preimage_attack
const LEAF_TYPE: [u8; 1] = [0x00];
const PARENT_TYPE: [u8; 1] = [0x01];
const ROOT_TYPE: [u8; 1] = [0x02];

pub const HASH_SIZE: usize = HASH_LENGTH;

/// `BLAKE2b` hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hash {
    hash: [u8; HASH_SIZE],
}

impl Hash {
    /// Hash data to form a leaf `Hash`.
    #[inline]
    pub fn from_leaf(data: &[u8]) -> Self {
        let length = data.len() as u64;

        let mut hasher = Hasher::new();
        hasher.update(&LEAF_TYPE);
        hasher.update(&u64_to_bytes(length));
        hasher.update(data);
        let hash = hasher.finalize().into();

        Self { hash }
    }

    /// Hash two `Hash` together to form a parent `Hash`.
    #[inline]
    pub fn from_hashes(left: &Hash, right: &Hash, length: u64) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(&PARENT_TYPE);
        hasher.update(&u64_to_bytes(length));
        hasher.update(&left.hash);
        hasher.update(&right.hash);
        let hash = hasher.finalize().into();

        Self { hash }
    }

    /// Hash a vector of `Root` nodes.
    #[inline]
    pub fn from_roots(roots: &[&Hash], lengths: &[u64]) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(&ROOT_TYPE);

        for (node, length) in roots.iter().zip(lengths.iter()) {
            hasher.update(&u64_to_bytes(*length));
            hasher.update(&node.hash);
        }
        let hash = hasher.finalize().into();

        Self { hash }
    }

    /// Returns a byte slice of this `Hash`.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.hash
    }

    /// Create `Hash` from hash bytes and supplied length.
    #[inline]
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        ensure!(data.len() == HASH_SIZE);
        let hash = data.try_into().unwrap();
        Ok(Self {
            hash,
        })
    }
}

impl Deref for Hash {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

#[inline]
fn u64_to_bytes(n: u64) -> [u8; size_of::<u64>()] {
    let mut size = [0u8; size_of::<u64>()];
    size.as_mut().write_u64::<LittleEndian>(n).unwrap();
    size
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;

    fn hex_bytes(hex: &str) -> Vec<u8> {
        hex::decode(hex).unwrap()
    }
    fn check_hash(hash: Hash, hex: &str) {
        println!("{}", hex::encode(hash.as_bytes()));
        assert_eq!(hash.as_bytes(), &hex_bytes(hex)[..]);
    }

    #[test]
    fn leaf_hash() {
        check_hash(
            Hash::from_leaf(&[]),
            "9d15372b1830735a0a2d05214669e3271a754aac46fe303b68bb3046013a0574",
        );
        check_hash(
            Hash::from_leaf(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
            "ca7f9e37f2b51ced7bce80abc43a753567dec65eefdef6a94c7f601974b59651",
        );
    }

    #[test]
    fn parent_hash() {
        let data1 = [0, 1, 2, 3, 4];
        let data2 = [42, 43, 44, 45, 46, 47, 48];
        let hash1 = Hash::from_leaf(&data1);
        let hash2 = Hash::from_leaf(&data2);
        let length = data1.len() as u64 + data2.len() as u64;
        check_hash(
            Hash::from_hashes(&hash1, &hash2, length),
            "41286ad3998a9dceba5e5c229fb20744e2e09c9555981af41e35974c8272ae3b",
        );
        check_hash(
            Hash::from_hashes(&hash2, &hash1, length),
            "a4a09813803617badb50b44a01c848d7d71d29808a78c28bff1dd779be92aa8d",
        );
    }

    #[test]
    fn root_hash() {
        let data1 = [0, 1, 2, 3, 4];
        let data2 = [42, 43, 44, 45, 46, 47, 48];
        let hash1 = Hash::from_leaf(&data1);
        let hash2 = Hash::from_leaf(&data2);
        check_hash(
            Hash::from_roots(
                &[&hash1, &hash2],
                &[data1.len() as u64, data2.len() as u64]),
            "9b5022ef9e4326beb4b9d0f007856ee2398dad10bac4673572736d811519d080",
        );
        check_hash(
            Hash::from_roots(
                &[&hash2, &hash1],
                &[data2.len() as u64, data1.len() as u64]),
            "d26188a66e8e124d0e01493e372b63a8816f476176bebaf62d46395a328816f5",
        );
    }
}
