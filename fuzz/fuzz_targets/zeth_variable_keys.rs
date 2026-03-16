#![no_main]

use std::{
    collections::BTreeMap,
    panic::{catch_unwind, AssertUnwindSafe},
};

use alloy_primitives::{B256, Bytes};
use alloy_trie::{HashBuilder, Nibbles};
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use zeth_mpt::{CachedTrie, Trie};

#[derive(Debug, Arbitrary)]
enum Op {
    Insert { key: Vec<u8>, value: Vec<u8> },
    Remove { key: Vec<u8> },
}

#[derive(Debug, Arbitrary)]
struct Input {
    ops: Vec<Op>,
}

fn model_root(model: &BTreeMap<Vec<u8>, Bytes>) -> B256 {
    let mut hash_builder = HashBuilder::default();
    for (key, value) in model {
        hash_builder.add_leaf(Nibbles::unpack(key), value);
    }
    hash_builder.root()
}

fuzz_target!(|input: Input| {
    let mut trie = Trie::default();
    let mut cached_trie = CachedTrie::default();
    let mut model = BTreeMap::<Vec<u8>, Bytes>::new();

    for op in &input.ops {
        match op {
            Op::Insert { key, value } => {
                if value.is_empty() {
                    let mut trie_probe = trie.clone();
                    let mut cached_probe = cached_trie.clone();
                    assert!(
                        catch_unwind(AssertUnwindSafe(|| trie_probe.insert(key, Bytes::new())))
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

                let value = Bytes::copy_from_slice(value);
                trie.insert(key, value.clone());
                cached_trie.insert(key, value.clone());
                model.insert(key.clone(), value);
            }
            Op::Remove { key } => {
                trie.remove(key);
                cached_trie.remove(key);
                model.remove(key);
            }
        }

        let trie_root = trie.hash_slow();
        let cached_root = cached_trie.hash();
        assert_eq!(trie_root, cached_root, "Trie root != CachedTrie root");

        let expected = model_root(&model);
        assert_eq!(trie_root, expected, "zeth-mpt root != HashBuilder root");
    }
});
