//! Implementation of the simple MPT.
use crate::trie::TrieNode::{Digest, Leaf};
use crate::trie::Trie;
use super::nodes::{DigestNode, LeafNode};
use alloy_primitives::map::{FbBuildHasher, HashMap};
use alloy_primitives::{B256, Bytes};
use alloy_trie::{EMPTY_ROOT_HASH, Nibbles};

/// Added only to make an IDE happy. It is defined in alloy_primitives::map
pub type B256Map<V> = HashMap<B256, V, FbBuildHasher<32>>;

impl Trie {
    /// Creates empty trie.
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Inserts a value under the `path` key. Overrides previous values if exists.
    pub fn insert(&mut self, path: Nibbles, value: Bytes) {
        match self.root.as_mut() {
            Some(root) => root.insert(path, value),
            None => {
                self.root = Some(Leaf(LeafNode {
                    path,
                    value,
                    hash: None,
                }))
            }
        }
    }

    /// Gets a value assosiated with a `path` key
    pub fn get(&self, path: Nibbles) -> Option<&Bytes> {
        if self.root.is_none() {
            None
        } else {
            self.root.as_ref().unwrap().get(path)
        }
    }

    /// Returns a root hash of the trie
    pub fn hash(&mut self) -> B256 {
        match self.root.as_mut() {
            Some(root) => root.hash(),
            None => EMPTY_ROOT_HASH,
        }
    }

    /// Removes an element from the trie. If the element does not exist, it does nothing.
    pub fn remove(&mut self, path: Nibbles) {
        match self.root.as_mut() {
            Some(root) => match root {
                Leaf(leaf) => {
                    if path.eq(&leaf.path) {
                        self.root = None;
                    }
                }
                _ => root.remove(path),
            },
            None => return,
        }
    }

    /// Build a trie according to elements encoded in a hash->value map starting from the `root_hash`
    pub fn reveal_from_rlp(root_hash: B256, rlp_rep_map: &B256Map<Bytes>) -> Self {
        let mut trie = Trie::new();
        if root_hash == EMPTY_ROOT_HASH {
            return trie;
        }
        trie.root = Some(Digest(DigestNode {
            value: root_hash,
            hash: Some(root_hash),
            path: Nibbles::default(),
        }));
        trie.root.as_mut().unwrap().reveal(rlp_rep_map);
        trie
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Bytes, hex};
    use alloy_trie::Nibbles;

    #[test]
    fn basic_and_extension_node_test() {
        let mut trie = Trie::new();
        trie.insert(
            Nibbles::unpack(hex!("0x12343123").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 1, 2, 3]),
        );
        trie.insert(
            Nibbles::unpack(hex!("0x12353123").to_vec()),
            Bytes::from([1, 2, 3, 5, 3, 1, 2, 3]),
        );
        trie.insert(
            Nibbles::unpack(hex!("0x12354123").to_vec()),
            Bytes::from([1, 2, 3, 5, 4, 1, 2, 3]),
        );
        trie.insert(
            Nibbles::unpack(hex!("0x12343223").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 2, 2, 3]),
        );
        trie.insert(
            Nibbles::unpack(hex!("0x12343023").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 0, 2, 3]),
        );
        // Add to top of an extension node. Common prefix is empty.
        trie.insert(
            Nibbles::unpack(hex!("0x22343223").to_vec()),
            Bytes::from([2, 2, 3, 4, 3, 2, 2, 3]),
        );
        // Add to an extension node. The extension node path reminder length is 1.
        trie.insert(
            Nibbles::unpack(hex!("0x12743223").to_vec()),
            Bytes::from([1, 2, 7, 4, 3, 2, 2, 3]),
        );
        // Add to an extension node. The extension node path length is 1.
        trie.insert(
            Nibbles::unpack(hex!("0x12345223").to_vec()),
            Bytes::from([1, 2, 3, 4, 5, 2, 2, 3]),
        );

        assert_eq!(
            *trie
                .get(Nibbles::unpack(hex!("0x12343123").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 4, 3, 1, 2, 3])
        );
        assert_eq!(
            *trie
                .get(Nibbles::unpack(hex!("0x12353123").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 5, 3, 1, 2, 3])
        );
        assert_eq!(
            *trie
                .get(Nibbles::unpack(hex!("0x12354123").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 5, 4, 1, 2, 3])
        );
        assert_eq!(
            *trie
                .get(Nibbles::unpack(hex!("0x12343223").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 4, 3, 2, 2, 3])
        );
        assert_eq!(
            *trie
                .get(Nibbles::unpack(hex!("0x12343023").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 4, 3, 0, 2, 3])
        );
        assert_eq!(
            *trie
                .get(Nibbles::unpack(hex!("0x22343223").to_vec()))
                .unwrap(),
            Bytes::from([2, 2, 3, 4, 3, 2, 2, 3])
        );
        assert_eq!(
            *trie
                .get(Nibbles::unpack(hex!("0x12743223").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 7, 4, 3, 2, 2, 3])
        );
        assert_eq!(
            *trie
                .get(Nibbles::unpack(hex!("0x12345223").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 4, 5, 2, 2, 3])
        );
    }

    #[test]
    fn basic_and_extension_node_middle_path_test() {
        let mut trie = Trie::new();
        trie.insert(
            Nibbles::unpack(hex!("0x12343123").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 1, 2, 3]),
        );
        trie.insert(
            Nibbles::unpack(hex!("0x12353123").to_vec()),
            Bytes::from([1, 2, 3, 5, 3, 1, 2, 3]),
        );
        trie.insert(
            Nibbles::unpack(hex!("0x12354123").to_vec()),
            Bytes::from([1, 2, 3, 5, 4, 1, 2, 3]),
        );
        trie.insert(
            Nibbles::unpack(hex!("0x12343223").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 2, 2, 3]),
        );
        // Add to an extension node in the middle of the exstension node path.
        trie.insert(
            Nibbles::unpack(hex!("0x11343223").to_vec()),
            Bytes::from([1, 1, 3, 4, 3, 2, 2, 3]),
        );

        assert_eq!(
            *trie
                .get(Nibbles::unpack(hex!("0x12343123").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 4, 3, 1, 2, 3])
        );
        assert_eq!(
            *trie
                .get(Nibbles::unpack(hex!("0x12353123").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 5, 3, 1, 2, 3])
        );
        assert_eq!(
            *trie
                .get(Nibbles::unpack(hex!("0x12354123").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 5, 4, 1, 2, 3])
        );
        assert_eq!(
            *trie
                .get(Nibbles::unpack(hex!("0x12343223").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 4, 3, 2, 2, 3])
        );
        assert_eq!(
            *trie
                .get(Nibbles::unpack(hex!("0x11343223").to_vec()))
                .unwrap(),
            Bytes::from([1, 1, 3, 4, 3, 2, 2, 3])
        );
        // Override the value of the extension node.
        trie.insert(
            Nibbles::unpack(hex!("0x11343223").to_vec()),
            Bytes::from([1, 1, 3, 4, 3, 2, 2, 9]),
        );
        assert_eq!(
            *trie
                .get(Nibbles::unpack(hex!("0x11343223").to_vec()))
                .unwrap(),
            Bytes::from([1, 1, 3, 4, 3, 2, 2, 9])
        );
    }

    #[test]
    fn remove_test() {
        let mut trie = Trie::new();
        trie.insert(
            Nibbles::unpack(hex!("0x12343123").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 1, 2, 3]),
        );
        trie.insert(
            Nibbles::unpack(hex!("0x12353123").to_vec()),
            Bytes::from([1, 2, 3, 5, 3, 1, 2, 3]),
        );
        trie.insert(
            Nibbles::unpack(hex!("0x12354123").to_vec()),
            Bytes::from([1, 2, 3, 5, 4, 1, 2, 3]),
        );
        trie.insert(
            Nibbles::unpack(hex!("0x12343223").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 2, 2, 3]),
        );
        trie.insert(
            Nibbles::unpack(hex!("0x12343023").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 0, 2, 3]),
        );

        println!("{}", trie);
        trie.remove(Nibbles::unpack(hex!("0x12343123").to_vec()));
        assert_eq!(trie.get(Nibbles::unpack(hex!("0x12343123").to_vec())), None);
        println!("{}", trie);
        trie.remove(Nibbles::unpack(hex!("0x12353123").to_vec()));
        assert_eq!(trie.get(Nibbles::unpack(hex!("0x12353123").to_vec())), None);
        println!("{}", trie);
        trie.remove(Nibbles::unpack(hex!("0x12354123").to_vec()));
        assert_eq!(trie.get(Nibbles::unpack(hex!("0x12354123").to_vec())), None);
        println!("{}", trie);
        trie.remove(Nibbles::unpack(hex!("0x12343223").to_vec()));
        assert_eq!(trie.get(Nibbles::unpack(hex!("0x12343223").to_vec())), None);
        println!("{}", trie);
        trie.remove(Nibbles::unpack(hex!("0x12343023").to_vec()));
        assert_eq!(trie.get(Nibbles::unpack(hex!("0x12343023").to_vec())), None);
        println!("{}", trie);
        // TODO: Change it when hash implementation is done.
        assert_eq!(trie.to_string(), "Trie { EMPTY }");
    }
}
