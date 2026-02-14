#![allow(missing_docs)]

#[cfg(test)]
mod tests {
    use alloy_primitives::Signature;
    use reth_chainspec::ChainSpec;
    use reth_evm_ethereum::EthEvmConfig;
    use stateless::{
        stateless_validation_with_trie, validation::stateless_validation, Genesis, StatelessInput,
        UncompressedPublicKey,
    };
    use ref_mpt_state::SimpleSparseState;
    use std::{fs::File, path::PathBuf, sync::Arc};

    /// Recovers the uncompressed public key from a transaction signature and signing hash.
    fn recover_public_key(sig: &Signature, hash: alloy_primitives::B256) -> UncompressedPublicKey {
        let r = sig.r();
        let s = sig.s();
        let mut sig_bytes = [0u8; 64];
        sig_bytes[..32].copy_from_slice(&r.to_be_bytes::<32>());
        sig_bytes[32..].copy_from_slice(&s.to_be_bytes::<32>());

        let signature =
            k256::ecdsa::Signature::from_slice(&sig_bytes).expect("valid signature bytes");
        let recid = k256::ecdsa::RecoveryId::new(sig.v(), false);

        let key = k256::ecdsa::VerifyingKey::recover_from_prehash(
            hash.as_slice(),
            &signature,
            recid,
        )
        .expect("valid public key recovery");

        let point = key.to_encoded_point(false);
        let mut bytes = [0u8; 65];
        bytes.copy_from_slice(point.as_bytes());
        UncompressedPublicKey(bytes)
    }

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

        let public_keys: Vec<UncompressedPublicKey> = input
            .block
            .body
            .transactions
            .iter()
            .map(|tx| {
                let sig = tx.signature();
                let hash = tx.signature_hash();
                recover_public_key(sig, hash)
            })
            .collect();

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
