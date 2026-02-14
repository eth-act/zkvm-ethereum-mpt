use std::sync::Arc;

use benchmarks::recover_signers;
use reth_ethereum_primitives as _;
use criterion::{Criterion, criterion_group, criterion_main};
use reth_chainspec::ChainSpec;
use reth_evm_ethereum::EthEvmConfig;
use reth_stateless::{
    Genesis, StatelessInput, stateless_validation_with_trie, validation::stateless_validation,
};
use ref_mpt_state::SimpleSparseState;
use zeth_mpt_state::SparseState;

fn stateless_validation_benchmark(c: &mut Criterion) {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test_data/rpc_block_23439901.json"
    );

    let Ok(file) = std::fs::File::open(path) else {
        eprintln!("Skipping benchmark: test data not found at {path}");
        return;
    };

    let input: StatelessInput = serde_json::from_reader(file).expect("failed to parse test data");

    let genesis = Genesis {
        config: input.chain_config.clone(),
        ..Default::default()
    };
    let chain_spec: Arc<ChainSpec> = Arc::new(genesis.into());
    let evm_config = EthEvmConfig::new(chain_spec.clone());

    let public_keys = recover_signers(input.block.body.transactions.iter())
        .expect("failed to recover signers");

    let mut group = c.benchmark_group("stateless_validation");

    group.bench_function("reth_default", |b| {
        b.iter(|| {
            stateless_validation(
                input.block.clone(),
                public_keys.clone(),
                input.witness.clone(),
                chain_spec.clone(),
                evm_config.clone(),
            )
            .expect("stateless validation failed")
        });
    });

    group.bench_function("sparse_state", |b| {
        b.iter(|| {
            stateless_validation_with_trie::<SparseState, ChainSpec, EthEvmConfig>(
                input.block.clone(),
                public_keys.clone(),
                input.witness.clone(),
                chain_spec.clone(),
                evm_config.clone(),
            )
            .expect("stateless validation failed")
        });
    });

    group.bench_function("simple_sparse_state", |b| {
        b.iter(|| {
            stateless_validation_with_trie::<SimpleSparseState, ChainSpec, EthEvmConfig>(
                input.block.clone(),
                public_keys.clone(),
                input.witness.clone(),
                chain_spec.clone(),
                evm_config.clone(),
            )
            .expect("stateless validation failed")
        });
    });

    group.finish();
}

criterion_group!(benches, stateless_validation_benchmark);
criterion_main!(benches);
