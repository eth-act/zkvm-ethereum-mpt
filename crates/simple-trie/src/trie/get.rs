//! Implementation of getting an element from the MPT trie according to the element's path value.
use crate::trie::TrieNode::{Branch, Digest, Leaf};
use super::nodes::{BranchNode, DigestNode, LeafNode, TrieNode};
use alloy_primitives::Bytes;
use alloy_trie::Nibbles;

impl LeafNode {
    fn get(&self, path: Nibbles) -> Option<&Bytes> {
        if self.path == path {
            Some(&self.value)
        } else {
            None
        }
    }
}

impl BranchNode {
    fn get(&self, path: Nibbles) -> Option<&Bytes> {
        // It is only possible in case when the `self.path` is a prefix of `path`,
        // otherwise return None.
        let common_prefix_len = self.path.common_prefix_length(&path);
        if common_prefix_len == self.path.len() {
            if let Some(child) = self.children.get(path[common_prefix_len] as usize)
            {
                child.get(path.slice(common_prefix_len + 1..))
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl DigestNode {
    fn get(&self, path: Nibbles) -> Option<&Bytes> {
        // Disallow access to the digest node child, but allow when accessing a path which is
        // a prefix of the digest node path.
        if path.common_prefix_length(&self.path) < self.path.len() {
            return None;
        } else {
            panic!("MPT: Unresolved node access")
        }
    }
}

impl TrieNode {
    pub(super) fn get(&self, path: Nibbles) -> Option<&Bytes> {
        match self {
            Leaf(leaf) => leaf.get(path),
            Branch(branch) => branch.get(path),
            Digest(digest) => digest.get(path),
        }
    }
}
