#![allow(missing_docs)]

#[cfg(test)]
mod tests {
    use guest_libs::senders::recover_signers;
    use reth_chainspec::ChainSpec;
    use reth_evm_ethereum::EthEvmConfig;
    use reth_stateless::{
        stateless_validation_with_trie, validation::stateless_validation, Genesis, StatelessInput,
    };
    use ref_mpt_state::SimpleSparseState;
    use std::{fs::File, path::PathBuf, sync::Arc};

    #[test]
    fn stateless_validation_test() {
        let mut input_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        input_path.push("../test_data/rpc_block_23439901.json");
        if !input_path.exists() {
            eprintln!("skipping stateless_validation_test: missing fixture {input_path:?}");
            return;
        }

        let input = serde_json::from_reader::<_, StatelessInput>(
            File::open(input_path).expect("failed to open test input"),
        )
        .expect("failed to parse stateless input");

        let genesis = Genesis {
            config: input.chain_config.clone(),
            ..Default::default()
        };
        let chain_spec: Arc<ChainSpec> = Arc::new(genesis.into());
        let evm_config = EthEvmConfig::new(chain_spec.clone());

        let public_keys =
            recover_signers(input.block.body.transactions.iter()).expect("recovering signers");

        let reth_result = stateless_validation(
            input.block.clone(),
            public_keys.clone(),
            input.witness.clone(),
            chain_spec.clone(),
            evm_config.clone(),
        )
        .expect("reth stateless validation error");

        let simple_result =
            stateless_validation_with_trie::<SimpleSparseState, ChainSpec, EthEvmConfig>(
                input.block,
                public_keys,
                input.witness,
                chain_spec,
                evm_config,
            )
            .expect("simple sparse stateless validation error");

        assert_eq!(reth_result, simple_result);
    }
}
