#![allow(dead_code)]

//! You can represent a binary tree in a simple flat list using the following
//! structure:
//!
//! ```text
//!       3
//!   1       5
//! 0   2   4   6  ...
//! ```
//!
//! Each number represents an **index** in a flat list. So a tree:
//!
//! ```text
//!       A
//!   B       C
//! D   E   F   G  ...
//! ```
//!
//! would be represented as a list: `[D B E A F C G]`
//!
//! Furthermore, indexes `0`, `2`, `4`, `6` are on **depth** `0`.
//! Indexes `1`, `5`, `9` on depth `1`. And so forth.
//!
//! ```text
//! depth = 2  ^        3
//! depth = 1  |    1       5
//! depth = 0  |  0   2   4   6  ...
//! ```
//!
//! In some cases it is also useful to calculate an **offset**.
//! Indexes `0`, `1`, `3`, `7` have an offset `0`:
//!
//! ```text
//!                 (7)
//!        (3)
//!   (1)       5
//! (0)   2   4   6      ...
//! ```
//!
//! `2`, `5`, `11`, `23` offset `1`:
//!
//! ```text
//!                  7
//!        3                  (11)
//!   1        (5)        9          13
//! 0   (2)   4   6    10   12    14    15
//! ```
//!
//! This module exposes a series of functions to help you build and maintain
//! this data structure.

/// Returns the flat-tree of the tree node at the specified depth and offset.
#[inline]
pub const fn index(depth: u64, offset: u64) -> u64 {
    (offset << (depth + 1)) | ((1 << depth) - 1)
}

/// Returns the depth of a node.
#[inline]
pub const fn depth(i: u64) -> u64 {
    // Count trailing `1`s of the binary representation of the number.
    (!i).trailing_zeros() as u64
}

/// Returns the offset of a node.
#[inline]
pub fn offset(i: u64) -> u64 {
    let depth = self::depth(i);
    if is_even(i) {
        i / 2
    } else {
        i >> (depth + 1)
    }
}

/// Returns the parent of a node with a depth.
#[inline]
pub fn parent(i: u64) -> u64 {
    let depth = self::depth(i);
    index(depth + 1, offset(i) >> 1)
}

/// Returns only the left child of a node.
#[inline]
pub fn left_child(i: u64) -> Option<u64> {
    let depth = self::depth(i);
    if is_even(i) {
        None
    } else if depth == 0 {
        Some(i)
    } else {
        Some(index(depth - 1, offset(i) << 1))
    }
}

/// Returns only the right child of a node.
#[inline]
pub fn right_child(i: u64) -> Option<u64> {
    let depth = self::depth(i);
    if is_even(i) {
        None
    } else if depth == 0 {
        Some(i)
    } else {
        Some(index(depth - 1, (offset(i) << 1) + 1))
    }
}

/// Returns the right most node in the tree that the node spans.
#[inline]
pub fn right_span(i: u64) -> u64 {
    let depth = self::depth(i);
    if depth == 0 {
        i
    } else {
        (offset(i) + 1) * (2 << depth) - 2
    }
}

/// Returns the left most node in the tree that it spans.
#[inline]
pub fn left_span(i: u64) -> u64 {
    let depth = self::depth(i);
    if depth == 0 {
        i
    } else {
        offset(i) * (2 << depth)
    }
}

/// Returns the left and right most nodes in the tree that the node spans.
#[inline]
pub fn spans(i: u64) -> (u64, u64) {
    (left_span(i), right_span(i))
}

/// Returns how many nodes are in the tree that the node spans.
#[inline]
pub const fn count(i: u64) -> u64 {
    let depth = self::depth(i);
    (2 << depth) - 1
}

#[inline]
const fn is_even(num: u64) -> bool {
    (num & 1) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_even() {
        assert_eq!(is_even(0), true);
        assert_eq!(is_even(1), false);
        assert_eq!(is_even(2), true);
        assert_eq!(is_even(3), false);
    }

    #[test]
    fn test_parent_gt_int32() {
        assert_eq!(parent(10_000_000_000), 10_000_000_001);
    }

    #[test]
    fn test_child_to_parent_to_child() {
        let mut child = 0;
        for _ in 0..50 {
            child = parent(child)
        }
        assert_eq!(child, 1_125_899_906_842_623);
        for _ in 0..50 {
            child = left_child(child).expect("no valid number returned");
        }
        assert_eq!(child, 0);
    }

    #[test]
    fn test_flat_tree() {
        assert_eq!(index(0, 0), 0);
        assert_eq!(index(0, 1), 2);
        assert_eq!(index(0, 2), 4);
        assert_eq!(index(1, 2), 9);
        assert_eq!(index(1, 3), 13);
        assert_eq!(index(2, 1), 11);
        assert_eq!(index(2, 2), 19);
        assert_eq!(index(3, 0), 7);
        assert_eq!(index(3, 1), 23);

        assert_eq!(depth(0), 0);
        assert_eq!(depth(1), 1);
        assert_eq!(depth(2), 0);
        assert_eq!(depth(3), 2);
        assert_eq!(depth(4), 0);

        assert_eq!(offset(0), 0);
        assert_eq!(offset(1), 0);
        assert_eq!(offset(2), 1);
        assert_eq!(offset(3), 0);
        assert_eq!(offset(4), 2);

        assert_eq!(parent(0), 1);
        assert_eq!(parent(1), 3);
        assert_eq!(parent(2), 1);
        assert_eq!(parent(3), 7);
        assert_eq!(parent(4), 5);

        assert_eq!(left_child(0), None);
        assert_eq!(left_child(1), Some(0));
        assert_eq!(left_child(3), Some(1));

        assert_eq!(right_child(0), None);
        assert_eq!(right_child(1), Some(2));
        assert_eq!(right_child(3), Some(5));

        assert_eq!(right_span(0), 0);
        assert_eq!(right_span(1), 2);
        assert_eq!(right_span(3), 6);
        assert_eq!(right_span(23), 30);
        assert_eq!(right_span(27), 30);

        assert_eq!(left_span(0), 0);
        assert_eq!(left_span(1), 0);
        assert_eq!(left_span(3), 0);
        assert_eq!(left_span(23), 16);
        assert_eq!(left_span(27), 24);

        assert_eq!(spans(0), (0, 0));
        assert_eq!(spans(1), (0, 2));
        assert_eq!(spans(3), (0, 6));
        assert_eq!(spans(23), (16, 30));
        assert_eq!(spans(27), (24, 30));

        assert_eq!(count(0), 1);
        assert_eq!(count(1), 3);
        assert_eq!(count(3), 7);
        assert_eq!(count(5), 3);
        assert_eq!(count(23), 15);
        assert_eq!(count(27), 7);
    }
}
