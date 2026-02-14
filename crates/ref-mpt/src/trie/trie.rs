//! Implementation of the simple MPT for state/storage trie.
use super::nodes::{DigestNode, LeafNode};
use crate::trie::Trie;
use crate::trie::TrieNode::{Digest, Leaf};
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
    use alloc::string::ToString;
    use alloy_primitives::{Bytes, hex, keccak256};
    use alloy_trie::{HashBuilder, Nibbles};
    use std::collections::BTreeMap;
    use std::{println, vec};
    use std::vec::Vec;

    fn simple_trie_root(entries: &BTreeMap<B256, Bytes>) -> B256 {
        let mut trie = Trie::new();
        for (key, value) in entries {
            trie.insert(*key, value.clone());
        }
        trie.hash()
    }

    fn hash_builder_root(entries: &BTreeMap<B256, Bytes>) -> B256 {
        let mut hash_builder = HashBuilder::default();
        for (key, value) in entries {
            hash_builder.add_leaf(Nibbles::unpack(*key), value);
        }
        hash_builder.root()
    }

    fn assert_roots_match(entries: &BTreeMap<B256, Bytes>) {
        assert_eq!(simple_trie_root(entries), hash_builder_root(entries));
    }

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

    #[test]
    fn b256_overwrite_and_idempotent_remove() {
        let mut trie = Trie::new();
        let key = B256::repeat_byte(0x42);
        let value1 = Bytes::from([1_u8, 2, 3]);
        let value2 = Bytes::from([9_u8, 8, 7, 6]);

        trie.insert(key, value1);
        let root_after_first_insert = trie.hash();

        trie.insert(key, value2.clone());
        let root_after_overwrite = trie.hash();
        assert_ne!(root_after_overwrite, root_after_first_insert);
        assert_eq!(trie.get(key), Some(&value2));

        trie.insert(key, value2);
        assert_eq!(trie.hash(), root_after_overwrite);

        trie.remove(key);
        let root_after_remove = trie.hash();
        assert_eq!(root_after_remove, EMPTY_ROOT_HASH);

        trie.remove(key);
        assert_eq!(trie.hash(), root_after_remove);
    }

    #[test]
    fn unknown_key_get_and_remove_are_safe() {
        let known_key1 = B256::repeat_byte(0x01);
        let known_key2 = B256::repeat_byte(0x02);
        let unknown_key = B256::repeat_byte(0x03);
        let mut trie = Trie::new();

        trie.insert(known_key1, Bytes::from([0xAA]));
        trie.insert(known_key2, Bytes::from([0xBB]));
        let root_before = trie.hash();

        assert_eq!(trie.get(unknown_key), None);
        trie.remove(unknown_key);
        assert_eq!(trie.hash(), root_before);
    }

    #[test]
    fn insertion_order_independence() {
        let entries = [
            (keccak256([0_u8]), Bytes::from([1_u8, 2])),
            (keccak256([1_u8]), Bytes::from([3_u8, 4, 5])),
            (keccak256([2_u8]), Bytes::from([6_u8])),
            (keccak256([3_u8]), Bytes::from([7_u8, 8, 9, 10])),
            (keccak256([4_u8]), Bytes::from([11_u8, 12])),
        ];

        let mut forward = Trie::new();
        for (key, value) in entries.iter() {
            forward.insert(*key, value.clone());
        }
        let forward_root = forward.hash();

        let mut reverse = Trie::new();
        for (key, value) in entries.iter().rev() {
            reverse.insert(*key, value.clone());
        }
        let reverse_root = reverse.hash();

        let ordered_map: BTreeMap<_, _> = entries.into_iter().collect();
        assert_eq!(forward_root, reverse_root);
        assert_eq!(forward_root, hash_builder_root(&ordered_map));
    }

    #[test]
    fn randomized_differential_root_equivalence() {
        let mut model = BTreeMap::<B256, Bytes>::new();

        for case in 0_u8..8 {
            model.clear();
            for step in 0_u8..48 {
                let key = keccak256([case, step, 0xA5]);
                if step % 3 == 0 {
                    model.remove(&key);
                } else {
                    let len = 1 + ((case as usize + step as usize) % 64);
                    let value: Vec<u8> = (0..len)
                        .map(|i| (i as u8) ^ case.wrapping_mul(17) ^ step.wrapping_mul(29))
                        .collect();
                    model.insert(key, Bytes::from(value));
                }

                assert_roots_match(&model);
            }
        }
    }

    #[test]
    fn value_size_boundaries_match_hash_builder() {
        for len in [31_usize, 32, 33] {
            let mut entries = BTreeMap::new();
            entries.insert(keccak256([len as u8, 1_u8]), Bytes::from(vec![0x11; len]));
            entries.insert(keccak256([len as u8, 2_u8]), Bytes::from(vec![0x22; len]));
            entries.insert(keccak256([len as u8, 3_u8]), Bytes::from(vec![0x33; len]));
            assert_roots_match(&entries);
        }
    }
}
