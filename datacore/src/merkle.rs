use anyhow::{Result, ensure};
use std::mem::size_of;
use std::io::{Cursor, Read};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::merkle_tree_stream::{HashMethods, MerkleTreeStream};
use crate::hash::{Hash, HASH_SIZE};

pub use crate::merkle_tree_stream::Node as NodeTrait;

pub const NODE_SIZE: usize = 2 * size_of::<u64>() + HASH_SIZE;

/// [Merkle] node.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Node {
    index: u64,
    hash: Hash,
    length: u64,
}

impl Node {
    /// Deserialize [Node].
    #[inline]
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut rdr = Cursor::new(data);
        let index = rdr.read_u64::<LittleEndian>()?;
        let length = rdr.read_u64::<LittleEndian>()?;
        let mut hash_bytes = [0u8; HASH_SIZE];
        rdr.read_exact(&mut hash_bytes)?;
        let hash = Hash::from_bytes(&hash_bytes)?;
        Ok(Self {
            index,
            hash,
            length,
        })
    }

    /// Serialize [Node].
    #[inline]
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(NODE_SIZE);
        data.write_u64::<LittleEndian>(self.index)?;
        data.write_u64::<LittleEndian>(self.length)?;
        data.extend_from_slice(self.hash.as_bytes());
        ensure!(data.len() == NODE_SIZE);
        Ok(data)
    }
}

impl NodeTrait<Hash> for Node {
    #[inline]
    fn new(index: u64, hash: Hash, length: u64) -> Self {
        Self {
            index,
            hash,
            length,
        }
    }
    #[inline]
    fn index(&self) -> u64 {
        self.index as u64
    }
    #[inline]
    fn hash(&self) -> &Hash {
        &self.hash
    }
    #[inline]
    fn len(&self) -> u64 {
        self.length
    }
}

#[derive(Debug, Clone)]
struct H;

impl HashMethods for H {
    type Hash = Hash;
    type Node = Node;

    #[inline]
    fn leaf(&self, data: &[u8]) -> Self::Hash {
        Hash::from_leaf(data)
    }

    #[inline]
    fn parent(&self, left: &Self::Node, right: &Self::Node) -> Self::Hash {
        let length = left.length + right.length;
        Hash::from_hashes(&left.hash, &right.hash, length as u64)
    }
}

/// MerkleTreeStream for [Core].
///
/// [Core]: crate::core::Core
#[derive(Debug, Clone)]
pub struct Merkle {
    stream: MerkleTreeStream<H>,
}

impl Merkle {
    /// Create a new [Merkle].
    #[inline]
    pub fn new() -> Self {
        Self::from_roots(vec![])
    }

    /// Create a [Merkle] from root [Node]s.
    #[inline]
    pub fn from_roots(roots: Vec<Node>) -> Self {
        Self {
            stream: MerkleTreeStream::new(H, roots),
        }
    }

    /// Access the next item.
    #[inline]
    pub fn next(&mut self, data: Hash, length: u64) {
        self.stream.next(data, length);
    }

    /// Get the roots vector.
    #[inline]
    pub fn roots(&self) -> &Vec<Node> {
        self.stream.roots()
    }

    /// Get a vector of roots `Hash`'s'.
    #[inline]
    pub fn roots_hashes(&self) -> Vec<&Hash> {
        self.stream.roots().iter()
            .map(|node| &node.hash)
            .collect()
    }

    /// Get number of blocks.
    #[inline]
    pub fn blocks(&self) -> u64 {
        self.stream.blocks()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init() {
        Merkle::new();
    }

    #[test]
    fn node() {
        let mut merkle = Merkle::new();
        merkle.next(Hash::from_leaf("a".as_bytes()), 1);
        let node = merkle.roots().get(0).unwrap();
        let node2 = Node::from_bytes(&node.to_bytes().unwrap()).unwrap();
        assert_eq!(node2, *node);
    }

    #[test]
    fn next() {
        let mut merkle = Merkle::new();
        merkle.next(Hash::from_leaf("a".as_bytes()), 1);
        merkle.next(Hash::from_leaf("b".as_bytes()), 1);
        merkle.next(Hash::from_leaf("c".as_bytes()), 1);
        assert_eq!(merkle.blocks(), 3);
    }

    #[test]
    fn next_long_data() {
        let mut merkle = Merkle::new();
        let data1 = "hello_world".as_bytes();
        let data2 = vec![7u8; 1024];
        merkle.next(Hash::from_leaf(data1), data1.len() as u64);
        merkle.next(Hash::from_leaf(&data2), data2.len() as u64);
        assert_eq!(merkle.blocks(), 2);
    }

    #[test]
    fn roots_full() {
        let mut merkle = Merkle::new();
        merkle.next(Hash::from_leaf("a".as_bytes()), 1);
        merkle.next(Hash::from_leaf("b".as_bytes()), 1);
        merkle.next(Hash::from_leaf("c".as_bytes()), 1);
        merkle.next(Hash::from_leaf("d".as_bytes()), 1);
        let roots = merkle.roots();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots.get(0).unwrap().index(), 3);
    }
    #[test]
    fn roots() {
        let mut merkle = Merkle::new();
        merkle.next(Hash::from_leaf("a".as_bytes()), 1);
        merkle.next(Hash::from_leaf("b".as_bytes()), 1);
        merkle.next(Hash::from_leaf("c".as_bytes()), 1);
        let roots = merkle.roots();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots.get(0).unwrap().index(), 1);
        assert_eq!(roots.get(1).unwrap().index(), 4);
    }
}
