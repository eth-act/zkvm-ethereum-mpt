//! A sparse Simple Merkle Patricia trie implementation.
#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod trie;

pub use alloy_primitives::B256;
pub use alloy_trie::Nibbles;
pub use trie::B256Map;
pub use trie::Trie;
