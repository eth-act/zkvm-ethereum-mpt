mod display;
mod get;
mod hash;
mod insert;
mod remove;
mod reveal;
mod rlp;
mod trie;
mod children;
mod nodes;

use std::fmt::Debug;
use nodes::TrieNode;
pub use trie::B256Map;


/// Implements an Merkle Patricia Trie with 3 nodes' types (leaf, branch and digest)
#[derive(Debug, Clone)]
pub struct Trie {
    root: Option<TrieNode>,
}
