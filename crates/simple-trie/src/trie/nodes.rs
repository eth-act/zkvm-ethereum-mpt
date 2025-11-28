//! Definition of 3 node's types building the trie.

//! For optimization purpose and to simplify the implementation, we do not define an extension node.
//! This additional node type is implemented by extending the branch and the digest nodes' types with a path parameter.
//! It greatly simplifies the implementation of all trie modification and encoding algorithms.
use alloy_primitives::{Bytes, B256};
use crate::Nibbles;
pub(super) use crate::trie::children::BranchNodeChildrenArray;

#[derive(Debug, Clone)]
pub(crate) struct BranchNode {
    pub(crate) children: BranchNodeChildrenArray,
    pub(crate) path: Nibbles,
    pub(crate) hash: Option<B256>,
}

#[derive(Debug, Clone)]
pub(crate) struct LeafNode {
    pub(crate) path: Nibbles,
    pub(crate) value: Bytes,
    pub(crate) hash: Option<B256>,
}

#[derive(Debug, Clone)]
pub(crate) struct DigestNode {
    pub(crate) path: Nibbles,
    pub(crate) value: B256,
    pub(crate) hash: Option<B256>,
}

#[derive(Debug, Clone)]
pub(crate) enum TrieNode {
    Branch(BranchNode),
    Leaf(LeafNode),
    Digest(DigestNode),
}