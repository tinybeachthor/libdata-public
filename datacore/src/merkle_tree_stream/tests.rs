use std::iter;
use std::collections::HashSet;
use quickcheck::quickcheck;

use crypto_hash::{hex_digest, Algorithm};
use super::{
    DefaultNode, HashMethods, MerkleTreeStream, Node, flat_tree,
};

struct H;
impl HashMethods for H {
    type Hash = Vec<u8>;
    type Node = DefaultNode<Vec<u8>>;

    fn leaf(&self, data: &[u8]) -> Self::Hash {
        hex_digest(Algorithm::SHA256, &data).as_bytes().to_vec()
    }

    fn parent(&self, a: &Self::Node, b: &Self::Node) -> Self::Hash {
        let mut buf = Vec::with_capacity(a.hash().len() + b.hash().len());
        buf.extend_from_slice(a.hash());
        buf.extend_from_slice(b.hash());
        hex_digest(Algorithm::SHA256, &buf).as_bytes().to_vec()
    }
}

#[test]
fn mts_one_node() {
    let roots = Vec::new();
    let mut mts = MerkleTreeStream::new(H, roots);
    let data = b"hello";
    mts.next(H.leaf(data), data.len() as u64);

    // check node
    let n = mts.roots.pop().unwrap();
    assert_eq!(5, n.len());
    assert_eq!(0, n.index());

    let expected =
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
    assert_eq!(expected.as_bytes(), n.hash());
}

#[test]
fn mts_more_nodes() {
    let roots = Vec::new();
    let mut mts = MerkleTreeStream::new(H, roots);
    mts.next(H.leaf(b"a"), 1);
    mts.next(H.leaf(b"b"), 1);

    //   r
    //  / \
    // a   b
    assert_eq!(1, mts.roots().len());

    // check root node
    let expected_r =
        "62af5c3cb8da3e4f25061e829ebeea5c7513c54949115b1acc225930a90154da";
    {
        let rs = mts.roots();
        assert_eq!(1, rs.len());

        let r = &rs[0];
        assert_eq!(expected_r.as_bytes(), r.hash());
    }

    // add a third one
    mts.next(H.leaf(b"c"), 1);

    //   r    c
    //  / \
    // a   b
    assert_eq!(2, mts.roots().len());
    {
        // r's hash hasn't changed
        let rs = mts.roots();
        assert_eq!(2, rs.len());
        let r = &rs[0];
        assert_eq!(expected_r.as_bytes(), r.hash());

        let expected_c =
            "2e7d2c03a9507ae265ecf5b5356885a53393a2029d241394997265a1a25aefc6";
        let c = &rs[1];
        assert_eq!(expected_c.as_bytes(), c.hash());
    }

    // add a fourth one
    mts.next(H.leaf(b"d"), 1);

    //       t
    //     /   \
    //   r       s
    //  / \     / \
    // a   b   c   d
    assert_eq!(1, mts.roots().len());
    {
        let rs = mts.roots();
        let t = &rs[0];
        let expected_t =
            "58c89d709329eb37285837b042ab6ff72c7c8f74de0446b091b6a0131c102cfd";
        assert_eq!(expected_t.as_bytes(), t.hash());
    }
}

fn build_mts(data: &[Vec<u8>]) -> MerkleTreeStream<H> {
    let roots = vec![];
    let mut mts = MerkleTreeStream::new(H, roots);
    for bs in data {
        mts.next(H.leaf(bs), bs.len() as u64);
    }
    mts
}

fn all_children(index: u64) -> Box<dyn Iterator<Item = u64>> {
    let self_ = iter::once(index);
    match (flat_tree::left_child(index), flat_tree::right_child(index)) {
        (Some(left), Some(right)) => {
            Box::new(
                self_.chain(all_children(left)).chain(all_children(right)))
        },
        _ => Box::new(self_),
    }
}

#[test]
fn mts_is_deterministic() {
    fn prop(data: Vec<Vec<u8>>) -> bool {
        let mts1 = build_mts(&data);
        let mts2 = build_mts(&data);

        mts1.roots() == mts2.roots()
    }
    quickcheck(prop as fn(Vec<Vec<u8>>) -> bool);
}

#[test]
fn roots_have_no_parent() {
    fn prop(data: Vec<Vec<u8>>) -> bool {
        let len = data.len() as u64;
        let mts = build_mts(&data);
        let roots = mts.roots();

        let root_parents: HashSet<_> = roots.iter()
            .map(|root| flat_tree::parent(root.index()))
            .collect();
        root_parents.iter().all(|parent| *parent >= len)
    }
    quickcheck(prop as fn(Vec<Vec<u8>>) -> bool);
}

#[test]
fn hashes_change_when_data_is_changed() {
    fn prop(
        first_block: Vec<u8>,
        rest: Vec<Vec<u8>>,
        n: usize,
        update: Vec<u8>,
        ) -> bool {
        // Make sure there is at least one block to replace
        let mut data = rest;
        data.insert(0, first_block);

        let n = n % data.len();
        let orig_mts = build_mts(&data);
        let mut new_data = data.clone();
        let update_is_same = new_data[n] == update;
        new_data[n] = update;
        let new_mts = build_mts(&new_data);

        update_is_same || new_mts.roots() != orig_mts.roots()
    }
    quickcheck(prop as fn(Vec<u8>, Vec<Vec<u8>>, usize, Vec<u8>) -> bool);
}

#[test]
fn mts_new_with_nodes() {
    let roots = vec![DefaultNode {
        index: 0,
        hash: vec![],
        length: 4,
    }];
    let mts = MerkleTreeStream::new(H, roots);

    assert_eq!(mts.blocks(), 1);
}

struct XorHashMethods;
impl HashMethods for XorHashMethods {
    type Hash = Vec<u8>;
    type Node = DefaultNode<Vec<u8>>;

    fn leaf(&self, data: &[u8]) -> Self::Hash {
        // bitwise XOR the data into u8
        let hash = data.iter().fold(0, |acc, x| acc ^ x);
        vec![hash]
    }

    fn parent(&self, a: &Self::Node, b: &Self::Node) -> Self::Hash {
        let hash = Node::hash(a).iter()
            .chain(Node::hash(b).iter())
            .fold(0, |acc, x| acc ^ x);
        vec![hash]
    }
}

#[test]
fn xor_hash_example() {
    let mut mts = MerkleTreeStream::new(XorHashMethods, Vec::new());
    let data = b"hello";
    mts.next(XorHashMethods.leaf(data), data.len() as u64);
    let data = b"hashed";
    mts.next(XorHashMethods.leaf(data), data.len() as u64);
    let data = b"world";
    mts.next(XorHashMethods.leaf(data), data.len() as u64);

    // Constructed tree:
    //
    //   0(hello)-──┐
    //              1
    //   2(hashed)──┘
    //
    //   4(world)

    let xor_world = b"world".iter().fold(0, |acc, x| { acc ^ x });

    assert_eq!(mts.roots().len(), 2);
    assert_eq!(mts.roots()[0].index, 1);
    assert_eq!(mts.roots()[1].index, 4);

    let last_node = mts.roots().get(1).unwrap();
    assert_eq!(last_node.index, 4);
    assert_eq!(flat_tree::parent(last_node.index), 5);
    assert_eq!(last_node.length, 5);
    assert_eq!(last_node.hash, vec![xor_world]);
}
