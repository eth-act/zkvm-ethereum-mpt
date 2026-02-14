//! Implementation of the simple MPT for state/storage trie.
use super::nodes::{DigestNode, LeafNode};
use crate::trie::Trie;
use crate::trie::TrieNode::{Digest, Leaf};
use alloy_primitives::map::{FbBuildHasher, HashMap};
use alloy_primitives::{Bytes, B256};
use alloy_trie::{Nibbles, EMPTY_ROOT_HASH};

/// Added only to make an IDE happy. It is defined in alloy_primitives::map
pub type B256Map<V> = HashMap<B256, V, FbBuildHasher<32>>;

impl Trie {
    /// Creates empty trie.
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Inserts a value under the `key` key. Overrides previous values if exists.
    /// `key` must be a pre-hashed 32-byte key (state/storage trie key).
    pub fn insert(&mut self, key: B256, value: Bytes) {
        self.insert_path(Nibbles::unpack(key), value);
    }

    pub(crate) fn insert_path(&mut self, path: Nibbles, value: Bytes) {
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

    /// Gets a value associated with a pre-hashed 32-byte `key`.
    pub fn get(&self, key: B256) -> Option<&Bytes> {
        self.get_path(Nibbles::unpack(key))
    }

    pub(crate) fn get_path(&self, path: Nibbles) -> Option<&Bytes> {
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

    /// Removes an element from the trie by pre-hashed 32-byte `key`.
    pub fn remove(&mut self, key: B256) {
        self.remove_path(Nibbles::unpack(key));
    }

    pub(crate) fn remove_path(&mut self, path: Nibbles) {
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
    use alloy_primitives::{hex, keccak256, Bytes};
    use alloy_trie::Nibbles;

    #[test]
    fn basic_and_extension_node_test() {
        let mut trie = Trie::new();
        trie.insert_path(
            Nibbles::unpack(hex!("0x12343123").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 1, 2, 3]),
        );
        trie.insert_path(
            Nibbles::unpack(hex!("0x12353123").to_vec()),
            Bytes::from([1, 2, 3, 5, 3, 1, 2, 3]),
        );
        trie.insert_path(
            Nibbles::unpack(hex!("0x12354123").to_vec()),
            Bytes::from([1, 2, 3, 5, 4, 1, 2, 3]),
        );
        trie.insert_path(
            Nibbles::unpack(hex!("0x12343223").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 2, 2, 3]),
        );
        trie.insert_path(
            Nibbles::unpack(hex!("0x12343023").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 0, 2, 3]),
        );
        // Add to top of an extension node. Common prefix is empty.
        trie.insert_path(
            Nibbles::unpack(hex!("0x22343223").to_vec()),
            Bytes::from([2, 2, 3, 4, 3, 2, 2, 3]),
        );
        // Add to an extension node. The extension node path reminder length is 1.
        trie.insert_path(
            Nibbles::unpack(hex!("0x12743223").to_vec()),
            Bytes::from([1, 2, 7, 4, 3, 2, 2, 3]),
        );
        // Add to an extension node. The extension node path length is 1.
        trie.insert_path(
            Nibbles::unpack(hex!("0x12345223").to_vec()),
            Bytes::from([1, 2, 3, 4, 5, 2, 2, 3]),
        );

        assert_eq!(
            *trie
                .get_path(Nibbles::unpack(hex!("0x12343123").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 4, 3, 1, 2, 3])
        );
        assert_eq!(
            *trie
                .get_path(Nibbles::unpack(hex!("0x12353123").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 5, 3, 1, 2, 3])
        );
        assert_eq!(
            *trie
                .get_path(Nibbles::unpack(hex!("0x12354123").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 5, 4, 1, 2, 3])
        );
        assert_eq!(
            *trie
                .get_path(Nibbles::unpack(hex!("0x12343223").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 4, 3, 2, 2, 3])
        );
        assert_eq!(
            *trie
                .get_path(Nibbles::unpack(hex!("0x12343023").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 4, 3, 0, 2, 3])
        );
        assert_eq!(
            *trie
                .get_path(Nibbles::unpack(hex!("0x22343223").to_vec()))
                .unwrap(),
            Bytes::from([2, 2, 3, 4, 3, 2, 2, 3])
        );
        assert_eq!(
            *trie
                .get_path(Nibbles::unpack(hex!("0x12743223").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 7, 4, 3, 2, 2, 3])
        );
        assert_eq!(
            *trie
                .get_path(Nibbles::unpack(hex!("0x12345223").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 4, 5, 2, 2, 3])
        );
    }

    #[test]
    fn basic_and_extension_node_middle_path_test() {
        let mut trie = Trie::new();
        trie.insert_path(
            Nibbles::unpack(hex!("0x12343123").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 1, 2, 3]),
        );
        trie.insert_path(
            Nibbles::unpack(hex!("0x12353123").to_vec()),
            Bytes::from([1, 2, 3, 5, 3, 1, 2, 3]),
        );
        trie.insert_path(
            Nibbles::unpack(hex!("0x12354123").to_vec()),
            Bytes::from([1, 2, 3, 5, 4, 1, 2, 3]),
        );
        trie.insert_path(
            Nibbles::unpack(hex!("0x12343223").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 2, 2, 3]),
        );
        // Add to an extension node in the middle of the exstension node path.
        trie.insert_path(
            Nibbles::unpack(hex!("0x11343223").to_vec()),
            Bytes::from([1, 1, 3, 4, 3, 2, 2, 3]),
        );

        assert_eq!(
            *trie
                .get_path(Nibbles::unpack(hex!("0x12343123").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 4, 3, 1, 2, 3])
        );
        assert_eq!(
            *trie
                .get_path(Nibbles::unpack(hex!("0x12353123").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 5, 3, 1, 2, 3])
        );
        assert_eq!(
            *trie
                .get_path(Nibbles::unpack(hex!("0x12354123").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 5, 4, 1, 2, 3])
        );
        assert_eq!(
            *trie
                .get_path(Nibbles::unpack(hex!("0x12343223").to_vec()))
                .unwrap(),
            Bytes::from([1, 2, 3, 4, 3, 2, 2, 3])
        );
        assert_eq!(
            *trie
                .get_path(Nibbles::unpack(hex!("0x11343223").to_vec()))
                .unwrap(),
            Bytes::from([1, 1, 3, 4, 3, 2, 2, 3])
        );
        // Override the value of the extension node.
        trie.insert_path(
            Nibbles::unpack(hex!("0x11343223").to_vec()),
            Bytes::from([1, 1, 3, 4, 3, 2, 2, 9]),
        );
        assert_eq!(
            *trie
                .get_path(Nibbles::unpack(hex!("0x11343223").to_vec()))
                .unwrap(),
            Bytes::from([1, 1, 3, 4, 3, 2, 2, 9])
        );
    }

    #[test]
    fn remove_test() {
        let mut trie = Trie::new();
        trie.insert_path(
            Nibbles::unpack(hex!("0x12343123").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 1, 2, 3]),
        );
        trie.insert_path(
            Nibbles::unpack(hex!("0x12353123").to_vec()),
            Bytes::from([1, 2, 3, 5, 3, 1, 2, 3]),
        );
        trie.insert_path(
            Nibbles::unpack(hex!("0x12354123").to_vec()),
            Bytes::from([1, 2, 3, 5, 4, 1, 2, 3]),
        );
        trie.insert_path(
            Nibbles::unpack(hex!("0x12343223").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 2, 2, 3]),
        );
        trie.insert_path(
            Nibbles::unpack(hex!("0x12343023").to_vec()),
            Bytes::from([1, 2, 3, 4, 3, 0, 2, 3]),
        );

        println!("{}", trie);
        trie.remove_path(Nibbles::unpack(hex!("0x12343123").to_vec()));
        assert_eq!(
            trie.get_path(Nibbles::unpack(hex!("0x12343123").to_vec())),
            None
        );
        println!("{}", trie);
        trie.remove_path(Nibbles::unpack(hex!("0x12353123").to_vec()));
        assert_eq!(
            trie.get_path(Nibbles::unpack(hex!("0x12353123").to_vec())),
            None
        );
        println!("{}", trie);
        trie.remove_path(Nibbles::unpack(hex!("0x12354123").to_vec()));
        assert_eq!(
            trie.get_path(Nibbles::unpack(hex!("0x12354123").to_vec())),
            None
        );
        println!("{}", trie);
        trie.remove_path(Nibbles::unpack(hex!("0x12343223").to_vec()));
        assert_eq!(
            trie.get_path(Nibbles::unpack(hex!("0x12343223").to_vec())),
            None
        );
        println!("{}", trie);
        trie.remove_path(Nibbles::unpack(hex!("0x12343023").to_vec()));
        assert_eq!(
            trie.get_path(Nibbles::unpack(hex!("0x12343023").to_vec())),
            None
        );
        println!("{}", trie);
        // TODO: Change it when hash implementation is done.
        assert_eq!(trie.to_string(), "Trie { EMPTY }");
    }

    #[test]
    fn get_prefix_key_returns_none() {
        let mut trie = Trie::new();
        trie.insert_path(Nibbles::from_nibbles([1_u8, 2, 3]), Bytes::from([1_u8]));
        trie.insert_path(Nibbles::from_nibbles([1_u8, 2, 4]), Bytes::from([2_u8]));

        assert_eq!(trie.get_path(Nibbles::from_nibbles([1_u8, 2])), None);
    }

    #[test]
    fn remove_prefix_key_is_noop() {
        let mut trie = Trie::new();
        let key1 = Nibbles::from_nibbles([1_u8, 2, 3]);
        let key2 = Nibbles::from_nibbles([1_u8, 2, 4]);
        trie.insert_path(key1.clone(), Bytes::from([1_u8]));
        trie.insert_path(key2.clone(), Bytes::from([2_u8]));
        let hash_before = trie.hash();

        trie.remove_path(Nibbles::from_nibbles([1_u8, 2]));

        assert_eq!(trie.hash(), hash_before);
        assert_eq!(trie.get_path(key1), Some(&Bytes::from([1_u8])));
        assert_eq!(trie.get_path(key2), Some(&Bytes::from([2_u8])));
    }

    #[test]
    fn remove_last_child_from_revealed_one_child_branch_does_not_panic() {
        // RLP for a branch with one inlined leaf child at index 0 and an empty branch value.
        let root_rlp = Bytes::from(vec![
            0xd3, 0xc2, 0x20, 0x01, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
            0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
        ]);
        let root_hash = keccak256(&root_rlp);
        let mut map = B256Map::default();
        map.insert(root_hash, root_rlp);

        let mut trie = Trie::reveal_from_rlp(root_hash, &map);
        trie.remove_path(Nibbles::from_nibbles([0_u8]));
    }

    #[test]
    fn b256_key_api_roundtrip() {
        let mut trie = Trie::new();
        let key = B256::repeat_byte(0x11);
        let value = Bytes::from([7_u8]);

        trie.insert(key, value.clone());
        assert_eq!(trie.get(key), Some(&value));

        trie.remove(key);
        assert_eq!(trie.get(key), None);
    }
}
