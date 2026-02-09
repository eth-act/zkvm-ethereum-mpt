//! Building the MPT with the root hash and the trie nodes' values stored in a (hash)->(rlp encoded value) map.
//! This implementation stores hash if the nodes in a simple caching mechanism which greatly optimizes a
//! number of necessary hash calculations and node's rlp encodings.
use crate::trie::B256Map;
use crate::trie::TrieNode;
use crate::trie::TrieNode::{Branch, Digest, Leaf};
use alloy_primitives::{B256, Bytes};

impl TrieNode {
    fn set_cache(&mut self, hash: B256) {
        match self {
            Branch(branch) => {
                branch.hash = Some(hash);
            }
            Leaf(leaf) => {
                leaf.hash = Some(hash);
            }
            Digest(digest) => {
                digest.hash = Some(hash);
            }
        };
    }

    pub(crate) fn clear_cache(&mut self) {
        match self {
            Branch(branch) => {
                branch.hash = None;
            }
            Leaf(leaf) => {
                leaf.hash = None;
            }
            Digest(digest) => {
                digest.hash = None;
            }
        }
    }
}

impl TrieNode {
    pub(super) fn reveal(&mut self, rlp_rep_map: &B256Map<Bytes>) {
        match self {
            Leaf(_) => {}
            Branch(branch) => {
                for child in branch.children.iter_mut() {
                    match child {
                        Some(child) => {
                            child.reveal(rlp_rep_map);
                        }
                        None => {}
                    }
                }
            }
            Digest(digest) => match rlp_rep_map.get(&digest.value) {
                Some(rlp) => {
                    let mut node = TrieNode::decode(&mut &rlp[..])
                        .expect("MPT: Failed to decode trie node")
                        .expect("MPT: Empty trie node");

                    match node {
                        Digest(ref digest_node) => {
                            if digest_node.path.is_empty() {
                                // The digest value does not reveal anything but the hash.
                                return;
                            }
                        }
                        Branch(ref mut branch) => {
                            // The digest reveals to branch. Assign the digest's path to the branch.
                            branch.path = core::mem::take(&mut digest.path);
                        }
                        Leaf(_) => {}
                    }

                    // Set cache based on the hash of the digest node which reveals to non-digest or
                    // digest with a non-empty path. At this moment the digest hash should be cached.
                    node.set_cache(digest.hash());
                    node.reveal(rlp_rep_map);
                    *self = node;
                }
                None => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trie::Trie;
    use alloy_primitives::{hex, keccak256};
    use alloy_trie::Nibbles;

    #[test]
    fn reveal_from_rlp() {
        let state: Vec<Bytes> = {
            [
            Bytes::from(hex!("0xf869a0206aea581b220579a2b99819299dd32c7c28a420018ecb0bde93af007ad89a31b846f8440180a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a078c6cb5202685228bbcbfb992b1c4e116c7ec5ef11e25b8e92716cfc628ddd60")),
            Bytes::from(hex!("0xf869a037d65eaa92c6bc4c13a5ec45527f0c18ea8932588728769ec7aecfe6d9f32e42b846f8440180a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a0f57acd40259872606d76197ef052f3d35588dadf919ee1f0e3cb9b62d3f4b02c")),
            Bytes::from(hex!("0xf8b1a0c4b823e1deb537a6b4c41ecc9123e37753d61894f9dee7022b29c83088f69cfba00d1c2f6add00c6786d64a77d4136f71ef02f4a69307c77b663f32875ae8c7d9780a066a64e47bae97c0fccdc260c76b1c987c89560cb40e86ea17a1d5fd49e35bebe8080a039e4714d1eb6e1d5b21ca2bffd56333a7cd697596ff64317d1ae21ffd048e6ca808080808080a008be39f7c15cc06a7d863615397887281eadcbdb7907665d0683ca3c6383e6b0808080")),
            Bytes::from(hex!("0xf869a03f86c581c7d7b44eecbb92fd9e5867945ec1acdc0ea5bbabda21d17dddf06473b846f8440180a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a00345a365d2f4c5975b9f1599abe0a2ee76b7a3a731bc68781bd04c84e4858f50")),
            Bytes::from(hex!("0xf869a03d7dcb6a0ce5227c5379fc5b0e004561d7833b063355f69bfea3178f08fbaab4b846f8440180a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a09fb907ad9cb2872884a1e6839fcf89d229ef9b43df0511f58dbb26a1217ecb0d")),
            Bytes::from(hex!("0xf851808080a0de090f75dbe520ac527f21140ede3807a7dc416a0bae24c33dde9fe04300a08c808080808080808080a0f215e6bc9ca85972bc2488943dca80313a019f5eb569cc6ee3dc8c2af68734af808080")),
            Bytes::from(hex!("0x80")),
            Bytes::from(hex!("0xf851808080808080808080808080a031357c4a138624e300159fc631211a29d8373db4bdf59b80dad6e816593d0bcb8080a0b5790ff14363bee5d40c4a9fd9d6a515fc44683cc4d46666b4d9c775dded101780")),
            Bytes::from(hex!("0xf871a020601462093b5945d1676df093446790fd31b20e7b12a2e8e5e09d068109616bb84ef84c80880de0b6b3a7640000a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a0c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")),
            Bytes::from(hex!("0xf869a0209d57be05dd69371c4dd2e871bce6e9f4124236825bb612ee18a45e5675be51b846f8440180a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a06e49e66782037c0555897870e29fa5e552daf4719552131a0abce779daec0a5d"))
        ].to_vec()
        };

        let rlp_map: B256Map<Bytes> = state
            .iter()
            .map(|rlp| (keccak256(&rlp), rlp.clone()))
            .collect();
        let root_hash = B256::from(hex!(
            "0x5e5fc7fb30faa5cdc163023c4ce2dc8807601ec858dd2905738dad824d0a21ce"
        ));

        let mut trie = Trie::reveal_from_rlp(root_hash, &rlp_map);
        assert_eq!(trie.hash(), root_hash);

        let to_remove = Nibbles::from_nibbles([
            0, 3, 6, 0, 1, 4, 6, 2, 0, 9, 3, 11, 5, 9, 4, 5, 13, 1, 6, 7, 6, 13, 15, 0, 9, 3, 4, 4,
            6, 7, 9, 0, 15, 13, 3, 1, 11, 2, 0, 14, 7, 11, 1, 2, 10, 2, 14, 8, 14, 5, 14, 0, 9, 13,
            0, 6, 8, 1, 0, 9, 6, 1, 6, 11,
        ]);
        trie.remove(to_remove.to_owned());
        assert_ne!(trie.hash(), root_hash);

        trie.insert(to_remove, Bytes::from(hex!("0xf84c80880de0b6b3a7640000a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a0c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")));
        assert_eq!(trie.hash(), root_hash);
    }
}
