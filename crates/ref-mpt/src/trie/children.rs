//! Implementation of a 16-element branch node children array.
//! It stores an additional bit flag indicating which child is not empty. Only for an optimization purpose
use alloc::boxed::Box;
use crate::trie::TrieNode;
use core::slice::{Iter, IterMut};

#[derive(Debug, Clone, Default)]
pub(super) struct BranchNodeChildrenArray {
    children: [Option<Box<TrieNode>>; 16],
    flags: u16,
}

impl BranchNodeChildrenArray {
    #[inline]
    pub(super) fn new() -> Self {
        Self {
            children: [const { None }; 16],
            flags: 0,
        }
    }

    #[inline]
    pub(super) fn get(&self, idx: usize) -> &Option<Box<TrieNode>> {
        unsafe { self.children.get_unchecked(idx) }
    }

    #[inline]
    pub(super) fn get_mut(&mut self, idx: usize) -> Option<&mut Box<TrieNode>> {
        unsafe {
            if let Some(child) = self.children.get_unchecked_mut(idx) {
                Some(child)
            } else {
                None
            }
        }
    }

    #[inline]
    pub(super) fn insert(&mut self, idx: usize, node: Box<TrieNode>) {
        self.children[idx] = Some(node);
        self.flags |= 1 << idx;
    }

    #[inline]
    pub(super) fn remove(&mut self, idx: usize) {
        self.children[idx] = None;
        self.flags &= !(1 << idx);
    }

    #[inline]
    pub(super) fn is_empty(&self) -> bool {
        self.flags == 0
    }

    #[inline]
    pub(super) fn one_child_left(&mut self) -> Option<(usize, &mut Box<TrieNode>)> {
        if self.flags == 0 || self.flags & (self.flags - 1) != 0 {
            None
        } else {
            let idx = self.flags.trailing_zeros() as usize;
            Some((
                idx,
                self.children[idx]
                    .as_mut()
                    .expect("MPT: Inconsistent branch children flags"),
            ))
        }
    }

    #[inline]
    pub(super) fn iter_mut(&mut self) -> IterMut<'_, Option<Box<TrieNode>>> {
        self.children.iter_mut()
    }

    #[inline]
    pub(super) fn iter(&self) -> Iter<'_, Option<Box<TrieNode>>> {
        self.children.iter()
    }
}
