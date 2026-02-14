#![allow(unused_crate_dependencies, missing_docs)]

use benchmarks::{WitnessConfig, generate_hashed_post_state, generate_test_witness};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ref_mpt_state::SimpleSparseState;
use reth_stateless::StatelessTrie;
use zeth_mpt_state::SparseState;

fn bench_trie_new(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie_new");

    for num_accounts in [10, 100, 1000] {
        let data = generate_test_witness(&WitnessConfig {
            num_accounts,
            num_storage_accounts: 0,
            slots_per_account: 0,
        });

        group.bench_function(BenchmarkId::new("simple_sparse_state", num_accounts), |b| {
            b.iter(|| {
                SimpleSparseState::new(&data.witness, data.pre_state_root)
                    .expect("failed to create trie")
            });
        });

        group.bench_function(BenchmarkId::new("sparse_state", num_accounts), |b| {
            b.iter(|| {
                SparseState::new(&data.witness, data.pre_state_root)
                    .expect("failed to create trie")
            });
        });
    }

    group.finish();
}

fn bench_trie_account(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie_account");

    for num_accounts in [10, 100, 1000] {
        let data = generate_test_witness(&WitnessConfig {
            num_accounts,
            num_storage_accounts: 0,
            slots_per_account: 0,
        });

        group.bench_function(BenchmarkId::new("simple_sparse_state", num_accounts), |b| {
            let (trie, _) = SimpleSparseState::new(&data.witness, data.pre_state_root)
                .expect("failed to create trie");
            b.iter(|| {
                for addr in &data.addresses {
                    trie.account(*addr).expect("account lookup failed");
                }
            });
        });

        group.bench_function(BenchmarkId::new("sparse_state", num_accounts), |b| {
            let (trie, _) = SparseState::new(&data.witness, data.pre_state_root)
                .expect("failed to create trie");
            b.iter(|| {
                for addr in &data.addresses {
                    trie.account(*addr).expect("account lookup failed");
                }
            });
        });
    }

    group.finish();
}

fn bench_trie_storage(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie_storage");

    for num_slots in [10, 100] {
        let data = generate_test_witness(&WitnessConfig {
            num_accounts: 1,
            num_storage_accounts: 1,
            slots_per_account: num_slots,
        });

        group.bench_function(BenchmarkId::new("simple_sparse_state", num_slots), |b| {
            let (trie, _) = SimpleSparseState::new(&data.witness, data.pre_state_root)
                .expect("failed to create trie");
            // account() must be called first to populate the storage trie cache
            for (addr, _) in &data.storage_entries {
                trie.account(*addr).expect("account lookup failed");
            }
            b.iter(|| {
                for (addr, slots) in &data.storage_entries {
                    for (slot, _) in slots {
                        trie.storage(*addr, *slot).expect("storage lookup failed");
                    }
                }
            });
        });

        group.bench_function(BenchmarkId::new("sparse_state", num_slots), |b| {
            let (trie, _) = SparseState::new(&data.witness, data.pre_state_root)
                .expect("failed to create trie");
            // account() must be called first to populate the storage trie cache
            for (addr, _) in &data.storage_entries {
                trie.account(*addr).expect("account lookup failed");
            }
            b.iter(|| {
                for (addr, slots) in &data.storage_entries {
                    for (slot, _) in slots {
                        trie.storage(*addr, *slot).expect("storage lookup failed");
                    }
                }
            });
        });
    }

    group.finish();
}

fn bench_trie_calculate_state_root(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie_calculate_state_root");

    for num_accounts in [10, 100, 1000] {
        let data = generate_test_witness(&WitnessConfig {
            num_accounts,
            num_storage_accounts: 0,
            slots_per_account: 0,
        });
        let post_state = generate_hashed_post_state(&data, num_accounts / 2);

        group.bench_function(BenchmarkId::new("simple_sparse_state", num_accounts), |b| {
            b.iter(|| {
                let (mut trie, _) =
                    SimpleSparseState::new(&data.witness, data.pre_state_root)
                        .expect("failed to create trie");
                trie.calculate_state_root(post_state.clone())
                    .expect("calculate_state_root failed");
            });
        });

        group.bench_function(BenchmarkId::new("sparse_state", num_accounts), |b| {
            b.iter(|| {
                let (mut trie, _) = SparseState::new(&data.witness, data.pre_state_root)
                    .expect("failed to create trie");
                trie.calculate_state_root(post_state.clone())
                    .expect("calculate_state_root failed");
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_trie_new,
    bench_trie_account,
    bench_trie_storage,
    bench_trie_calculate_state_root
);
criterion_main!(benches);
