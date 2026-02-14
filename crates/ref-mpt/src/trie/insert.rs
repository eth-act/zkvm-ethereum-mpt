//! Inserting an element to MPT implementation for different node's types.
use alloc::boxed::Box;
use crate::trie::TrieNode::{Branch, Digest, Leaf};
use super::nodes::{BranchNode, DigestNode, LeafNode, TrieNode, BranchNodeChildrenArray};
use alloy_primitives::Bytes;
use alloy_trie::Nibbles;

impl BranchNode {
    fn new(
        path: Nibbles,
        child1_idx: usize,
        child1: TrieNode,
        child2_idx: usize,
        child2: TrieNode,
    ) -> Self {
        let mut children = BranchNodeChildrenArray::new();
        children.insert(child1_idx, Box::new(child1));
        children.insert(child2_idx, Box::new(child2));
        Self {
            path,
            children,
            hash: None,
        }
    }

    fn insert(&mut self, path: Nibbles, value: Bytes) {
        let common_prefix_len = self.path.common_prefix_length(&path);

        if common_prefix_len == self.path.len() {
            // Add the new value to as a child of the branch node.
            let new_idx = path.at(common_prefix_len);
            let maybe_child = self.children.get_mut(new_idx);

            match maybe_child {
                Some(child) => {
                    // If the child is not empty, recursively go into the branch. Consume the first
                    // path nibble as it encodes the branch index.
                    child.insert(path.slice(common_prefix_len + 1..), value);
                    return;
                }
                None => {
                    // If the index branch is empty, insert the new leaf there.
                    let new_leaf = Leaf(LeafNode {
                        path: path.slice(common_prefix_len + 1..),
                        value,
                        hash: None,
                    });
                    self.children.insert(new_idx, Box::new(new_leaf));
                }
            }
        } else {
            // Create a new branch node with a path equal to the common path.
            // Attach the new leaf and current branch to the new branch node,
            // adjusting the paths accordingly.
            let current_digest_idx = self.path.at(common_prefix_len);
            let new_leaf_idx = path.at(common_prefix_len);

            *self = BranchNode::new(
                path.slice(..common_prefix_len),
                current_digest_idx,
                Branch(BranchNode {
                    path: self.path.slice(common_prefix_len + 1..),
                    children: core::mem::take(&mut self.children),
                    hash: None,
                }),
                new_leaf_idx,
                Leaf(LeafNode {
                    path: path.slice(common_prefix_len + 1..),
                    value,
                    hash: None,
                }),
            );
        }
    }
}

impl TrieNode {
    pub(super) fn insert(&mut self, path: Nibbles, value: Bytes) {
        self.clear_cache();
        match self {
            Leaf(leaf) => {
                if path == leaf.path {
                    // Override leaf node value.
                    leaf.value = value;
                } else {
                    let common_prefix_len = leaf.path.common_prefix_length(&path);
                    // Adding a leaf to a leaf node.
                    // Create a new branch node with a path equal to the common path.
                    // Attach the leaves to the new branch node adjusting the leaves' paths.
                    let current_leaf_idx = leaf.path.at(common_prefix_len);
                    let new_leaf_idx = path.at(common_prefix_len);

                    *self = Branch(BranchNode::new(
                        path.slice(..common_prefix_len),
                        current_leaf_idx,
                        Leaf(LeafNode {
                            path: leaf.path.slice(common_prefix_len + 1..),
                            value: core::mem::take(&mut leaf.value),
                            hash: None,
                        }),
                        new_leaf_idx,
                        Leaf(LeafNode {
                            path: path.slice(common_prefix_len + 1..),
                            value,
                            hash: None,
                        }),
                    ));
                }
            }
            Branch(branch) => {
                branch.insert(path, value);
            }
            Digest(digest) => {
                let common_prefix_len = path.common_prefix_length(&digest.path);
                if common_prefix_len < digest.path.len() {
                    // Create a new branch node with a path equal to the common path.
                    // Attach the current node and the new node to the new branch adjusting their paths.
                    let current_digest_idx = digest.path.at(common_prefix_len);
                    let new_leaf_idx = path.at(common_prefix_len);

                    *self = Branch(BranchNode::new(
                        path.slice(..common_prefix_len),
                        current_digest_idx,
                        Digest(DigestNode {
                            path: digest.path.slice(common_prefix_len + 1..),
                            value: digest.value,
                            hash: None,
                        }),
                        new_leaf_idx,
                        Leaf(LeafNode {
                            path: path.slice(common_prefix_len + 1..),
                            value,
                            hash: None,
                        }),
                    ));
                    return;
                } else {
                    // Adding to an unresolved node is impossible.
                    panic!("MPT: Unresolved node access");
                }
            }
        }
    }
}
