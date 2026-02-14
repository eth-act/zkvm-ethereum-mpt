//! Hashing element implementation for different node's types of MPT.
use super::nodes::{BranchNode, DigestNode, LeafNode, TrieNode};
use crate::trie::rlp::encode_list_header;
use crate::trie::TrieNode::{Branch, Digest, Leaf};
use alloy_primitives::private::alloy_rlp::Encodable;
use alloy_primitives::{keccak256, B256};
use alloy_trie::nodes::encode_path_leaf;

impl TrieNode {
    pub(super) fn hash(&mut self) -> B256 {
        match self {
            Leaf(leaf) => leaf.hash(),
            Branch(branch) => branch.hash(),
            Digest(digest) => digest.hash(),
        }
    }
}

impl LeafNode {
    // Returns RLP encoding of the leaf node.
    // https://ethereum.org/pl/developers/docs/data-structures-and-encoding/patricia-merkle-trie/#optimization
    fn encode(&self) -> Vec<u8> {
        // Encode the path of the leaf. It is not RLP encoding.
        // It is encoding of the path according to
        // https://ethereum.org/pl/developers/docs/data-structures-and-encoding/patricia-merkle-trie/#specification
        let path = encode_path_leaf(&self.path, true);
        // Prepare RLP encoded list header with a pre-allocated vector buffer.
        // The list contains two elements, the encoded `path` and `value`
        // Warning: `.length()` computes the *RLP* representation length of the value it is called on.
        let mut out = encode_list_header(path.length() + self.value.length());

        path.encode(&mut out);
        self.value[..].encode(&mut out);
        out
    }

    // Returns hash of the leaf node.
    // Caches computed hash to avoid unnecessary recomputations.
    fn hash(&mut self) -> B256 {
        match self.hash {
            Some(hash) => hash,
            None => {
                //keccak256(self.encode())
                self.hash = Some(keccak256(self.encode()));
                self.hash.unwrap()
            }
        }
    }
}

impl BranchNode {
    // Returns RLP encoding of the branch node.
    // https://ethereum.org/pl/developers/docs/data-structures-and-encoding/patricia-merkle-trie/#optimization
    fn encode(&mut self) -> Vec<u8> {
        static EMPTY_NODE: u8 = 0x80;

        let mut encoded: Vec<u8> = Vec::default();

        for child in self.children.iter_mut() {
            if let Some(child) = child {
                match child.as_mut() {
                    Leaf(leaf) => {
                        encoded.append(&mut shorten_encoding(leaf.encode()));
                    }
                    Branch(branch) => {
                        encoded.append(&mut shorten_encoding(branch.encode()));
                    }
                    Digest(digest) => {
                        if digest.path.is_empty() {
                            digest.value.encode(&mut encoded);
                        } else {
                            digest.hash()[..].encode(&mut encoded);
                        }
                    }
                }
            } else {
                encoded.push(EMPTY_NODE);
            }
        }

        // Push an empty branch value.
        encoded.push(EMPTY_NODE);

        // TODO: Check performance of this appending
        let mut encoded_branch = encode_list_header(encoded.len());
        encoded_branch.append(&mut encoded);

        if self.path.is_empty() {
            encoded_branch
        } else {
            // In case when a branch has a path, return (the encoded path, hash of the branch encoding).
            let encoded_path = encode_path_leaf(&self.path, false);
            let mut encoded_branch_shortened = shorten_encoding(encoded_branch);

            // `encoded_branch_shortened` is already encoded so we need to use absolut length (`.len()`)
            // and append instead of encode.
            // Warning: `.length()` computes the *RLP* representation length of the value it is called on.
            let mut encoded_branch_with_path =
                encode_list_header(encoded_path.length() + encoded_branch_shortened.len());

            encoded_path.encode(&mut encoded_branch_with_path);
            encoded_branch_with_path.append(&mut encoded_branch_shortened);
            encoded_branch_with_path
        }
    }

    // Returns hash of the branch node.
    // Caches computed hash to avoid unnecessary recomputations.
    fn hash(&mut self) -> B256 {
        match self.hash {
            Some(hash) => hash,
            None => {
                //keccak256(self.encode())
                self.hash = Some(keccak256(self.encode()));
                self.hash.unwrap()
            }
        }
    }
}

impl DigestNode {
    fn encode(&self) -> Vec<u8> {
        if self.path.is_empty() {
            let mut encoded_digest = Vec::with_capacity(33);
            self.value.encode(&mut encoded_digest);
            encoded_digest
        } else {
            let encoded_path = encode_path_leaf(&self.path, false);
            let mut encoded_digest_with_path = encode_list_header(
                encoded_path.length() + 33, /* encoded keccak256 value is always 33 bytes length */
            );

            encoded_path.encode(&mut encoded_digest_with_path);
            self.value.encode(&mut encoded_digest_with_path);
            encoded_digest_with_path
        }
    }

    pub(super) fn hash(&mut self) -> B256 {
        match self.hash {
            Some(hash) => hash,
            None => {
                if self.path.is_empty() {
                    // When the digest node has no path, its hash is equal to its value.
                    self.hash = Some(self.value);
                    self.value
                } else {
                    self.hash = Some(keccak256(self.encode()));
                    self.hash.unwrap()
                }
            }
        }
    }
}

// Encodes a branch child node depending on the child data length.
#[inline]
fn shorten_encoding(b: Vec<u8>) -> Vec<u8> {
    if b.len() < 32 {
        b
    } else {
        let mut out: Vec<u8> = Vec::with_capacity(32);
        keccak256(b).encode(&mut out);
        out
    }
}

// Test cases from https://github.com/ipsilon/evmone/blob/31bf2116792032e572394e86cc99d6227e1e98b1/test/unittests/state_mpt_test.cpp#L59-L183
#[cfg(test)]
mod tests {
    use crate::trie::Trie;
    use alloy_primitives::private::alloy_rlp::Encodable;
    use alloy_primitives::{hex, keccak256, Bytes};
    use alloy_trie::{HashBuilder, Nibbles};
    use std::vec;

    #[test]
    fn test_leaf_node_example1() {
        let mut trie = Trie::new();
        trie.insert_path(Nibbles::unpack(hex!("010203")), Bytes::from("hello"));
        assert_eq!(
            trie.hash(),
            hex!("82c8fd36022fbc91bd6b51580cfd941d3d9994017d59ab2e8293ae9c94c3ab6e")
        );
    }

    #[test]
    fn test_branch_node_example1() {
        // A trie of single branch node and two leaf nodes with paths of length 2.
        // The branch node has leaf nodes at positions [4] and [5].
        // {4:1, 5:a}

        let value1 = Bytes::from("v___________________________1");
        let key1 = Nibbles::unpack(hex!("0x41"));
        let path1 = [0x4u8, 0x1];
        let encoded_path1 = vec![0x30u8 | path1[1]];
        let mut leaf_node1 = vec![];
        vec![Bytes::from(encoded_path1), value1.clone()].encode(&mut leaf_node1);
        assert_eq!(
            leaf_node1,
            hex!("df319d765f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f31")
        );

        let value2 = Bytes::from("v___________________________2");
        let key2 = Nibbles::unpack(hex!("0x5a"));
        let path2 = [0x5u8, 0xa];
        let encoded_path2 = vec![0x30u8 | path2[1]];
        let mut leaf_node2 = vec![];
        vec![Bytes::from(encoded_path2), value2.clone()].encode(&mut leaf_node2);
        assert_eq!(
            leaf_node2,
            hex!("df3a9d765f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f32")
        );

        let mut trie = Trie::new();
        trie.insert_path(key1, value1);
        trie.insert_path(key2, value2);
        assert_eq!(
            trie.hash(),
            hex!("1aaa6f712413b9a115730852323deb5f5d796c29151a60a1f55f41a25354cd26")
        );
    }

    #[test]
    fn test_branch_node_of_3() {
        // A trie of single branch node and three leaf nodes with paths of length 2.
        // The branch node has leaf nodes at positions [0], [1] and [2]. All leaves have path 0.
        // {0:0 1:0 2:0}

        let mut trie = Trie::new();
        trie.insert_path(Nibbles::unpack(hex!("0x00")), Bytes::from("X"));
        trie.insert_path(Nibbles::unpack(hex!("0x10")), Bytes::from("Y"));
        trie.insert_path(Nibbles::unpack(hex!("0x20")), Bytes::from("Z"));
        assert_eq!(
            trie.hash(),
            hex!("5c5154e8d108dcf8b9946c8d33730ec8178345ce9d36e6feed44f0134515482d")
        );
    }

    #[test]
    fn test_leaf_node_with_empty_path() {
        // Both inserted leaves have empty path in the end.
        // 0:{0:"X", 1:"Y"}
        let mut trie = Trie::new();
        trie.insert_path(Nibbles::unpack(hex!("0x00")), Bytes::from("X"));
        trie.insert_path(Nibbles::unpack(hex!("0x01")), Bytes::from("Y"));
        println!("{}", trie);
        assert_eq!(
            trie.hash(),
            hex!("0a923005d10fbd4e571655cec425db7c5091db03c33891224073a55d3abc2415")
        );
    }

    #[test]
    fn test_extension_node_example1() {
        // A trie of an extension node followed by a branch node with
        // two leaves with single nibble paths.
        // 5858:{4:1, 5:a}

        let value1 = Bytes::from("v___________________________1");
        let key1 = Nibbles::unpack(hex!("0x585841"));
        let _path1 = [0x5u8, 0x8, 0x5, 0x8, 0x4, 0x1];

        let value2 = Bytes::from("v___________________________2");
        let key2 = Nibbles::unpack(hex!("0x58585a"));
        let _path2 = [0x5u8, 0x8, 0x5, 0x8, 0x5, 0xa];

        let encoded_common_path = vec![0x00u8, 0x58, 0x58];

        // The hash of the branch node. See the branch_node_example test.
        let branch_node_hash =
            hex!("1aaa6f712413b9a115730852323deb5f5d796c29151a60a1f55f41a25354cd26");

        let mut extension_node = vec![];
        vec![
            Bytes::from(encoded_common_path),
            Bytes::from(branch_node_hash),
        ]
        .encode(&mut extension_node);
        assert_eq!(
            keccak256(extension_node),
            hex!("3eefc183db443d44810b7d925684eb07256e691d5c9cb13215660107121454f9")
        );

        let mut trie = Trie::new();
        trie.insert_path(key1, value1);
        trie.insert_path(key2, value2);
        assert_eq!(
            trie.hash(),
            hex!("3eefc183db443d44810b7d925684eb07256e691d5c9cb13215660107121454f9")
        );
    }

    #[test]
    fn test_extension_node_example2() {
        // A trie of an extension node followed by a branch node with
        // two leaves with longer paths.
        // 585:{8:41, 9:5a}

        let value1 = Bytes::from("v___________________________1");
        let key1 = Nibbles::unpack(hex!("0x585841"));
        let path1 = [0x5u8, 0x8, 0x5, 0x8, 0x4, 0x1];

        let value2 = Bytes::from("v___________________________2");
        let key2 = Nibbles::unpack(hex!("0x58595a"));
        let path2 = [0x5u8, 0x8, 0x5, 0x9, 0x5, 0xa];

        let common_path = [0x5u8, 0x8, 0x5];
        let encoded_path1 = vec![0x20u8, ((path1[4] << 4) | path1[5])];
        assert_eq!(hex::encode(&encoded_path1), "2041");
        let encoded_path2 = vec![0x20u8, ((path2[4] << 4) | path2[5])];
        assert_eq!(hex::encode(&encoded_path2), "205a");

        let mut node1 = Vec::new();
        vec![Bytes::from(encoded_path1), value1.clone()].encode(&mut node1);
        assert_eq!(
            hex::encode(&node1),
            "e18220419d765f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f31"
        );
        let mut node2 = Vec::new();
        vec![Bytes::from(encoded_path2), value2.clone()].encode(&mut node2);
        assert_eq!(
            hex::encode(&node2),
            "e182205a9d765f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f32"
        );

        let branch_node_hash =
            hex!("01746f8ab5a4cc5d6175cbd9ea9603357634ec06b2059f90710243f098e0ee82");

        let encoded_common_path = vec![
            0x10u8 | common_path[0],
            ((common_path[1] << 4) | common_path[2]),
        ];
        let mut extension_node = Vec::new();
        vec![
            Bytes::from(encoded_common_path),
            Bytes::from(branch_node_hash),
        ]
        .encode(&mut extension_node);
        assert_eq!(
            hex::encode(keccak256(extension_node)),
            "ac28c08fa3ff1d0d2cc9a6423abb7af3f4dcc37aa2210727e7d3009a9b4a34e8"
        );

        let mut trie = Trie::new();
        trie.insert_path(key1, value1);
        trie.insert_path(key2, value2);
        assert_eq!(
            trie.hash(),
            hex!("ac28c08fa3ff1d0d2cc9a6423abb7af3f4dcc37aa2210727e7d3009a9b4a34e8")
        );
    }

    #[test]
    fn test_branch_child_encoding_matches_hash_builder() {
        let mut trie = Trie::new();
        let mut hash_builder = HashBuilder::default();
        let entries = [
            (Nibbles::from_nibbles([0_u8, 0]), vec![1_u8]),
            (Nibbles::from_nibbles([0_u8, 1]), vec![2_u8]),
            (Nibbles::from_nibbles([1_u8, 0]), vec![3_u8]),
        ];

        for (path, value) in entries {
            trie.insert_path(path.clone(), Bytes::from(value.clone()));
            hash_builder.add_leaf(path, &value);
        }

        assert_eq!(trie.hash(), hash_builder.root());
    }
}
