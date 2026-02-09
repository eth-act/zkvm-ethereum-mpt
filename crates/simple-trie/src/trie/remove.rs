//! Removing an element from MPT implementation for different node's types.
use crate::trie::TrieNode::{Branch, Digest, Leaf};
use super::nodes::{BranchNode, LeafNode, TrieNode};
use alloy_trie::Nibbles;

impl BranchNode {
    #[inline]
    fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    // Checks if the only child left in the branch node and returns its reference and its index.
    #[inline]
    fn only_one_child_left(&mut self) -> Option<(usize, &mut Box<TrieNode>)> {
        self.children.one_child_left()
    }

    fn remove(&mut self, path: Nibbles) {
        let common_prefix_len = self.path.common_prefix_length(&path);
        if common_prefix_len == self.path.len() {
            let idx = path.at(common_prefix_len);
            let maybe_child = self.children.get_mut(idx);
            match maybe_child {
                Some(child) => {
                    // Enter the child recursively
                    child.remove(path.slice(common_prefix_len + 1..));
                    // If the leaf is removed or the branch child is empty,
                    // remove the child from the branch,
                    match child.as_mut() {
                        Leaf(leaf) => {
                            if leaf.path == path.slice(common_prefix_len + 1..) {
                                self.children.remove(idx);
                            }
                        }
                        Branch(branch) => {
                            if branch.is_empty() {
                                self.children.remove(idx);
                            }
                        }
                        Digest(_) => panic!("MPT: Unresolved node access"),
                    }
                }
                None => {}
            }
        }
    }
}

impl TrieNode {
    pub(super) fn remove(&mut self, path: Nibbles) {
        self.clear_cache();
        match self {
            Leaf(_) => {}
            Branch(branch) => {
                branch.remove(path);
                // If only one child left in the branch:
                // 1. Branch left -> prepend the parent path to the child branch. Remove parent.
                // 2. Leaf left -> prepend the branch path to the leaf node path and replace the branch
                // with the leaf.
                let mut branch_path = branch.path.clone();
                if let Some((child_idx, child)) = branch.only_one_child_left() {
                    match child.as_mut() {
                        Branch(child_branch) => {
                            let mut new_path = core::mem::take(&mut branch_path);
                            new_path.push_unchecked(child_idx as u8);
                            new_path = new_path.join(&mut child_branch.path);

                            *self = Branch(BranchNode {
                                children: core::mem::take(&mut child_branch.children),
                                path: new_path,
                                hash: None,
                            });
                        }
                        Leaf(child_leaf) => {
                            let mut new_path = branch_path;
                            new_path.push_unchecked(child_idx as u8);
                            new_path = new_path.join(&mut child_leaf.path);

                            *self = Leaf(LeafNode {
                                path: new_path,
                                value: core::mem::take(&mut child_leaf.value),
                                hash: None,
                            });
                        }
                        Digest(_) => panic!("MPT: Unresolved node access"),
                    }
                }
            }
            Digest(_) => panic!("MPT: Unresolved node access"),
        }
    }
}
