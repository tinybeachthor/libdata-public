mod flat_tree;

/// Functions that need to be implemented for `MerkleTreeStream`.
pub trait HashMethods {
    /// The type of hash returned from the hashing functions.
    type Hash;
    /// The Node type we'll iterate over.
    type Node: Node<Self::Hash>;

    /// Pass data through a hash function.
    fn leaf(&self, data: &[u8]) -> Self::Hash;
    /// Pass hashes through a hash function.
    fn parent(&self, a: &Self::Node, b: &Self::Node) -> Self::Hash;
}

/// Functions that need to be implemented for the Data that
/// `MerkleTreeStream` works with.
pub trait Node<H> {
    /// Create a new Node.
    fn new(index: u64, hash: H, length: u64) -> Self;
    /// Get the position at which the node was found.
    fn index(&self) -> u64;
    /// Get the hash contained in the node.
    fn hash(&self) -> &H;
    /// Get the length of the node.
    fn len(&self) -> u64;
}

/// Node representation.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd, Clone)]
pub struct DefaultNode<H> {
    /// Offset into the flat-tree data structure.
    pub index: u64,
    /// Hash.
    pub hash: H,
    /// Total size of all its child nodes combined.
    pub length: u64,
}

impl<H> Node<H> for DefaultNode<H> {
    #[inline]
    fn new(index: u64, hash: H, length: u64) -> Self {
        Self { index, hash, length }
    }
    #[inline]
    fn index(&self) -> u64 {
        self.index
    }
    #[inline]
    fn hash(&self) -> &H {
        &self.hash
    }
    #[inline]
    fn len(&self) -> u64 {
        self.length
    }
}

/// A stream that generates a merkle tree based on the incoming data.
#[derive(Debug, Clone)]
pub struct MerkleTreeStream<T: HashMethods> {
    handler: T,
    roots: Vec<T::Node>,
    blocks: u64,
}

impl<H: HashMethods> MerkleTreeStream<H> {
    /// Create a new MerkleTreeStream instance.
    #[inline]
    pub fn new(handler: H, roots: Vec<H::Node>) -> MerkleTreeStream<H> {
        let blocks = if !roots.is_empty() {
            // Cant panic because roots.len() > 0
            let root = roots.last().unwrap();
            1 + flat_tree::right_span(root.index()) / 2
        } else {
            0
        };

        MerkleTreeStream {
            handler,
            roots,
            blocks,
        }
    }

    /// Pass a string buffer through the flat-tree hash functions.
    #[inline]
    pub fn next(&mut self, hash: H::Hash, length: u64) {
        let index: u64 = 2 * self.blocks;
        self.blocks += 1;

        let node = H::Node::new(index, hash, length);
        self.roots.push(node);

        while self.roots.len() > 1 {
            let leaf = {
                let left = &self.roots[self.roots.len() - 2];
                let right = &self.roots[self.roots.len() - 1];

                let left_parent = flat_tree::parent(left.index());
                let right_parent = flat_tree::parent(right.index());
                if left_parent != right_parent {
                    break;
                }

                let hash = self.handler.parent(left, right);
                H::Node::new(
                    left_parent,
                    hash,
                    left.len() + right.len())
            };
            for _ in 0..2 {
                self.roots.pop();
            }
            self.roots.push(leaf);
        }
    }

    /// Get the roots vector.
    #[inline]
    pub fn roots(&self) -> &Vec<H::Node> {
        &self.roots
    }

    /// Get number of blocks
    #[inline]
    pub fn blocks(&self) -> u64 {
        self.blocks
    }
}

#[cfg(test)]
mod tests;
