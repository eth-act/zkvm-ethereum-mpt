#![no_main]

use alloy_primitives::Bytes;
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use zeth_mpt::{CachedTrie, Trie};

#[derive(Debug, Arbitrary)]
enum Op {
    Insert { key: [u8; 32], value: Vec<u8> },
    Remove { key: [u8; 32] },
}

#[derive(Debug, Arbitrary)]
struct Input {
    ops: Vec<Op>,
}

fuzz_target!(|input: Input| {
    let mut trie = Trie::default();

    for op in &input.ops {
        match op {
            Op::Insert { key, value } => {
                if value.is_empty() {
                    continue;
                }
                trie.insert(key.as_slice(), Bytes::copy_from_slice(value));
            }
            Op::Remove { key } => {
                trie.remove(key.as_slice());
            }
        }
    }

    let original_hash = trie.hash_slow();

    // Roundtrip through RLP: serialize -> deserialize
    let rlp_nodes = trie.rlp_nodes();
    let reconstructed = Trie::from_rlp(&rlp_nodes).expect("from_rlp should not fail");

    assert_eq!(reconstructed.hash_slow(), original_hash, "Trie hash changed after RLP roundtrip");
    assert_eq!(reconstructed, trie, "Trie structure changed after RLP roundtrip");

    // Also test CachedTrie roundtrip
    let mut cached_reconstructed =
        CachedTrie::from_rlp(&rlp_nodes).expect("CachedTrie from_rlp should not fail");
    assert_eq!(
        cached_reconstructed.hash(),
        original_hash,
        "CachedTrie hash changed after RLP roundtrip"
    );
});
