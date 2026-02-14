#![no_main]

use std::collections::BTreeMap;

use alloy_primitives::{B256, Bytes};
use alloy_trie::{HashBuilder, Nibbles};
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use ref_mpt::Trie;

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
    let mut trie = Trie::new();
    let mut model = BTreeMap::<B256, Bytes>::new();

    for op in &input.ops {
        match op {
            Op::Insert { key, value } => {
                if value.is_empty() {
                    continue;
                }
                let key = B256::from(*key);
                let value = Bytes::copy_from_slice(value);
                trie.insert(key, value.clone());
                model.insert(key, value);
            }
            Op::Remove { key } => {
                let key = B256::from(*key);
                trie.remove(key);
                model.remove(&key);
            }
        }
    }

    // Build expected root from HashBuilder (canonical Ethereum implementation)
    let mut hash_builder = HashBuilder::default();
    for (key, value) in &model {
        hash_builder.add_leaf(Nibbles::unpack(*key), value);
    }
    let expected = hash_builder.root();

    let actual = trie.hash();
    assert_eq!(actual, expected, "ref-mpt root != HashBuilder root");
});
