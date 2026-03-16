#![no_main]

use alloy_primitives::{B256, Bytes};
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use std::panic::{catch_unwind, AssertUnwindSafe};

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
    let mut ref_trie = ref_mpt::Trie::new();
    let mut zeth_trie = zeth_mpt::Trie::default();
    let mut cached_trie = zeth_mpt::CachedTrie::default();

    for op in &input.ops {
        match op {
            Op::Insert { key, value } => {
                if value.is_empty() {
                    let mut zeth_probe = zeth_trie.clone();
                    let mut cached_probe = cached_trie.clone();
                    assert!(
                        catch_unwind(AssertUnwindSafe(|| zeth_probe.insert(key, Bytes::new())))
                            .is_err(),
                        "zeth Trie accepted empty value insert"
                    );
                    assert!(
                        catch_unwind(AssertUnwindSafe(|| cached_probe.insert(key, Bytes::new())))
                            .is_err(),
                        "zeth CachedTrie accepted empty value insert"
                    );
                    continue;
                }
                let b256_key = B256::from(*key);
                let bytes_value = Bytes::copy_from_slice(value);
                ref_trie.insert(b256_key, bytes_value.clone());
                zeth_trie.insert(key.as_slice(), bytes_value.clone());
                cached_trie.insert(key.as_slice(), bytes_value);
            }
            Op::Remove { key } => {
                let b256_key = B256::from(*key);
                ref_trie.remove(b256_key);
                zeth_trie.remove(key.as_slice());
                cached_trie.remove(key.as_slice());
            }
        }

        // Validate after each operation so transient divergences are not masked by later ops.
        let ref_root = ref_trie.hash();
        let zeth_root = zeth_trie.hash_slow();
        let cached_root = cached_trie.hash();

        assert_eq!(ref_root, zeth_root, "ref-mpt root != zeth-mpt Trie root");
        assert_eq!(ref_root, cached_root, "ref-mpt root != zeth-mpt CachedTrie root");
    }
});
