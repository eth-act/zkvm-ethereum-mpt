#![allow(unused_crate_dependencies)]
//! Benchmark utilities for comparing trie implementations.

use alloy_primitives::{Bytes, B256, KECCAK256_EMPTY, U256, keccak256, map::B256Map};
use alloy_rlp::Encodable;
use alloy_trie::{EMPTY_ROOT_HASH, HashBuilder, Nibbles, TrieAccount, proof::ProofRetainer};
use reth_primitives_traits::Account;
use stateless::ExecutionWitness;
use reth_trie_common::HashedPostState;
use std::collections::BTreeMap;

/// Configuration for generating a test witness.
#[derive(Debug)]
pub struct WitnessConfig {
    /// Total number of accounts in the trie.
    pub num_accounts: usize,
    /// How many of those accounts have non-empty storage.
    pub num_storage_accounts: usize,
    /// Number of storage slots per storage-bearing account.
    pub slots_per_account: usize,
}

/// Generated witness data for benchmarks.
#[derive(Debug)]
pub struct GeneratedWitness {
    /// The execution witness containing all trie proof nodes.
    pub witness: ExecutionWitness,
    /// Root hash of the generated account trie.
    pub pre_state_root: B256,
    /// Addresses of all generated accounts.
    pub addresses: Vec<alloy_primitives::Address>,
    /// For accounts with storage: (address, vec of (slot, value)) pairs.
    pub storage_entries: Vec<(alloy_primitives::Address, Vec<(U256, U256)>)>,
}

/// Create an address deterministically from an index.
fn make_address(i: usize) -> alloy_primitives::Address {
    let mut addr_bytes = [0u8; 20];
    let val = (i + 1) as u32;
    addr_bytes[16] = (val >> 24) as u8;
    addr_bytes[17] = (val >> 16) as u8;
    addr_bytes[18] = (val >> 8) as u8;
    addr_bytes[19] = val as u8;
    alloy_primitives::Address::from(addr_bytes)
}

/// Build a storage trie and return its root, proof nodes, and the slot key-value pairs.
pub fn generate_storage_trie(
    seed: usize,
    num_slots: usize,
) -> (B256, Vec<Bytes>, Vec<(U256, U256)>) {
    if num_slots == 0 {
        return (EMPTY_ROOT_HASH, Vec::new(), Vec::new());
    }

    let mut leaves = BTreeMap::new();
    let mut slot_pairs = Vec::new();

    for i in 0..num_slots {
        let slot = U256::from(seed * 1000 + i);
        let value = U256::from(i + 1);
        let hashed_key = keccak256(B256::from(slot));
        let nibbles = Nibbles::unpack(hashed_key);

        let mut buf = Vec::new();
        value.encode(&mut buf);

        leaves.insert(nibbles, buf);
        slot_pairs.push((slot, value));
    }

    // Retain proofs for all keys
    let proof_keys: Vec<Nibbles> = leaves.keys().cloned().collect();
    let mut hb = HashBuilder::default().with_proof_retainer(ProofRetainer::new(proof_keys));

    for (key, value) in &leaves {
        hb.add_leaf(key.clone(), value);
    }

    let root = hb.root();
    let proof_nodes: Vec<Bytes> = hb
        .take_proof_nodes()
        .into_nodes_sorted()
        .into_iter()
        .map(|(_, bytes)| bytes)
        .collect();

    (root, proof_nodes, slot_pairs)
}

/// Generate a complete test witness with accounts and optional storage.
pub fn generate_test_witness(config: &WitnessConfig) -> GeneratedWitness {
    let mut all_proof_nodes: Vec<Bytes> = Vec::new();
    let mut addresses = Vec::new();
    let mut storage_entries = Vec::new();
    let mut account_leaves = BTreeMap::new();

    for i in 0..config.num_accounts {
        let address = make_address(i);
        addresses.push(address);

        let (storage_root, storage_nodes) = if i < config.num_storage_accounts {
            let (root, nodes, slots) = generate_storage_trie(i, config.slots_per_account);
            storage_entries.push((address, slots));
            (root, nodes)
        } else {
            (EMPTY_ROOT_HASH, Vec::new())
        };

        // Add storage proof nodes to the flat list
        all_proof_nodes.extend(storage_nodes);

        let account = TrieAccount {
            nonce: i as u64,
            balance: U256::from((i + 1) * 1000),
            storage_root,
            code_hash: KECCAK256_EMPTY,
        };

        let hashed_address = keccak256(address);
        let nibbles = Nibbles::unpack(hashed_address);

        let mut buf = Vec::new();
        account.encode(&mut buf);

        account_leaves.insert(nibbles, buf);
    }

    // Build account trie
    let proof_keys: Vec<Nibbles> = account_leaves.keys().cloned().collect();
    let mut hb = HashBuilder::default().with_proof_retainer(ProofRetainer::new(proof_keys));

    for (key, value) in &account_leaves {
        hb.add_leaf(key.clone(), value);
    }

    let pre_state_root = hb.root();
    let account_proof_nodes: Vec<Bytes> = hb
        .take_proof_nodes()
        .into_nodes_sorted()
        .into_iter()
        .map(|(_, bytes)| bytes)
        .collect();

    // Merge account and storage proof nodes
    all_proof_nodes.extend(account_proof_nodes);

    let witness = ExecutionWitness {
        state: all_proof_nodes,
        codes: Vec::new(),
        keys: Vec::new(),
        headers: Vec::new(),
    };

    GeneratedWitness {
        witness,
        pre_state_root,
        addresses,
        storage_entries,
    }
}

/// Generate a [`HashedPostState`] that modifies `num_modified` accounts (balance changes).
pub fn generate_hashed_post_state(
    witness: &GeneratedWitness,
    num_modified: usize,
) -> HashedPostState {
    let mut accounts = B256Map::default();

    for (i, address) in witness.addresses.iter().take(num_modified).enumerate() {
        let hashed_address = keccak256(address);
        accounts.insert(
            hashed_address,
            Some(Account {
                nonce: i as u64,
                balance: U256::from((i + 1) * 2000), // Modified balance
                bytecode_hash: None,
            }),
        );
    }

    HashedPostState {
        accounts,
        storages: B256Map::default(),
    }
}
