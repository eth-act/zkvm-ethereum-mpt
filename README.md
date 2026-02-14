# zkvm-ethereum-mpt

Sparse Merkle Patricia Trie implementation for Ethereum, extracted and adapted from [zeth](https://github.com/boundless-xyz/zeth) for integration with Reth's `StatelessTrie` trait.

## Overview

This repository provides a sparse MPT implementation optimized for zero-knowledge virtual machines (zkVMs) and stateless Ethereum validation. The core code originates from [zeth](https://github.com/boundless-xyz/zeth) and has been adapted to implement the [`StatelessTrie` trait](https://github.com/paradigmxyz/reth/blob/ccb897f9a0d8967133d52347fa4d2e59a51a63f0/crates/stateless/src/trie.rs#L18-L44).

## Crates

| Crate | Directory | Description |
|---|---|---|
| `zeth-mpt` | `crates/zeth-mpt` | Production sparse MPT (`no_std`), extracted from zeth |
| `zeth-mpt-state` | `crates/zeth-mpt-state` | `StatelessTrie` impl over `zeth-mpt` (`no_std`) |
| `ref-mpt` | `crates/ref-mpt` | Reference simple MPT (`no_std`) |
| `ref-mpt-state` | `crates/ref-mpt-state` | `StatelessTrie` impl over `ref-mpt` (`no_std`) |

## Acknowledgments

Full credits of the MPT implementation to [zeth](https://github.com/boundless-xyz/zeth) authors and collaborators.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
