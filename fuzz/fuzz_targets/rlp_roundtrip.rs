#![no_main]

use alloy_primitives::{B256, Bytes};
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use ref_mpt::Trie as RefTrie;
use std::panic::{catch_unwind, AssertUnwindSafe};
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
    let mut ref_trie = RefTrie::new();

    for op in &input.ops {
        match op {
            Op::Insert { key, value } => {
                if value.is_empty() {
                    let mut zeth_probe = trie.clone();
                    assert!(
                        catch_unwind(AssertUnwindSafe(|| zeth_probe.insert(key, Bytes::new())))
                            .is_err(),
                        "zeth Trie accepted empty value insert"
                    );
                    continue;
                }
                let bytes_value = Bytes::copy_from_slice(value);
                trie.insert(key.as_slice(), bytes_value.clone());
                ref_trie.insert(B256::from(*key), bytes_value);
            }
            Op::Remove { key } => {
                trie.remove(key.as_slice());
                ref_trie.remove(B256::from(*key));
            }
        }
    }

    let original_hash = trie.hash_slow();
    let reference_hash = ref_trie.hash();
    assert_eq!(original_hash, reference_hash, "zeth-mpt root != ref-mpt root");

    // Roundtrip through RLP: serialize -> deserialize
    let rlp_nodes = trie.rlp_nodes();
    let reconstructed = Trie::from_rlp(&rlp_nodes).expect("from_rlp should not fail");

    assert_eq!(
        reconstructed.hash_slow(),
        reference_hash,
        "Trie hash changed after RLP roundtrip"
    );
    assert_eq!(reconstructed, trie, "Trie structure changed after RLP roundtrip");

    // Also test CachedTrie roundtrip
    let mut cached_reconstructed =
        CachedTrie::from_rlp(&rlp_nodes).expect("CachedTrie from_rlp should not fail");
    assert_eq!(
        cached_reconstructed.hash(),
        reference_hash,
        "CachedTrie hash changed after RLP roundtrip"
    );
});
