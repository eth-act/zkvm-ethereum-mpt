//! A sparse state implementation based on simple sparse trie.
use alloy_primitives::private::alloy_rlp;
use alloy_primitives::private::alloy_rlp::Decodable;
use alloy_primitives::{keccak256, map::hash_map::Entry, Address, Bytes, KECCAK256_EMPTY, U256};
use alloy_trie::{TrieAccount, EMPTY_ROOT_HASH};
use reth_errors::ProviderError;
use reth_revm::bytecode::Bytecode;
use reth_stateless::validation::StatelessValidationError;
use reth_stateless::{ExecutionWitness, StatelessTrie};
use reth_trie_common::HashedPostState;
use simple_trie::Trie;
use simple_trie::{B256Map, B256};
use std::cell::RefCell;

/// Implementation of a simple sparse state based on simple_trie
#[derive(Debug, Clone)]
pub struct SimpleSparseState {
    state: Trie,
    storages: RefCell<B256Map<Box<Trie>>>,
    rlp_by_digest: B256Map<Bytes>,
}

impl SimpleSparseState {
    /// Removes an account from the state.
    fn remove_account(&mut self, hashed_address: &B256) {
        self.state.remove(*hashed_address);
        self.storages.get_mut().remove(hashed_address);
    }

    /// Clears the storage of an account.
    fn clear_storage(&mut self, hashed_address: B256) -> &mut Box<Trie> {
        match self.storages.get_mut().entry(hashed_address) {
            Entry::Occupied(mut entry) => {
                entry.insert(Box::new(Trie::new()));
                entry
            }
            Entry::Vacant(entry) => entry.insert_entry(Box::new(Trie::new())),
        }
        .into_mut()
    }

    /// Returns a mutable version of the storage trie of the given account.
    fn storage_trie_mut(&mut self, hashed_address: B256) -> alloy_rlp::Result<&mut Box<Trie>> {
        let trie = match self.storages.get_mut().entry(hashed_address) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                // build the storage trie matching the storage root of the account
                let storage_root =
                    self.state
                        .get(hashed_address)
                        .map_or(EMPTY_ROOT_HASH, |value| {
                            alloy_rlp::decode_exact::<TrieAccount>(value)
                                .unwrap()
                                .storage_root
                        });
                entry.insert(Box::new(Trie::reveal_from_rlp(
                    storage_root,
                    &self.rlp_by_digest,
                )))
            }
        };

        Ok(trie)
    }
}

impl StatelessTrie for SimpleSparseState {
    fn new(
        witness: &ExecutionWitness,
        pre_state_root: B256,
    ) -> Result<(Self, B256Map<Bytecode>), StatelessValidationError>
    where
        Self: Sized,
    {
        // fist, hash all the RLP nodes once
        let rlp_by_digest: B256Map<_> = witness
            .state
            .iter()
            .map(|rlp| (keccak256(rlp), rlp.clone()))
            .collect();

        // construct the state trie from the witness data and the given state root
        let mut state = Trie::reveal_from_rlp(pre_state_root, &rlp_by_digest);

        // hash all the supplied bytecode
        let bytecode = witness
            .codes
            .iter()
            .map(|code| (keccak256(code), Bytecode::new_raw(code.clone())))
            .collect();

        debug_assert_eq!(state.hash(), pre_state_root);
        Ok((
            SimpleSparseState {
                state,
                storages: RefCell::new(B256Map::default()),
                rlp_by_digest,
            },
            bytecode,
        ))
    }

    fn account(&self, address: Address) -> Result<Option<TrieAccount>, ProviderError> {
        let hashed_address = keccak256(address);
        match self.state.get(hashed_address) {
            Some(value) => {
                match alloy_rlp::decode_exact(value.as_ref()) as Result<TrieAccount, _> {
                    Ok(account) => {
                        match self.storages.borrow_mut().entry(hashed_address) {
                            Entry::Vacant(entry) => {
                                if account.storage_root != EMPTY_ROOT_HASH {
                                    let t = Box::new(Trie::reveal_from_rlp(
                                        account.storage_root,
                                        &self.rlp_by_digest,
                                    ));
                                    entry.insert(t);
                                } else {
                                    entry.insert(Box::new(Trie::new()));
                                }
                            }
                            Entry::Occupied(_) => {}
                        }
                        Ok(Some(account))
                    }
                    Err(_) => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    fn storage(&self, address: Address, slot: U256) -> Result<U256, ProviderError> {
        match self.storages.borrow_mut().get(&keccak256(address)) {
            Some(storage_trie) => match storage_trie.get(keccak256(B256::from(slot))) {
                Some(value) => Ok(U256::decode(&mut &value[..]).unwrap()),
                None => Ok(U256::ZERO),
            },
            None => Ok(U256::ZERO),
        }
    }

    fn calculate_state_root(
        &mut self,
        state: HashedPostState,
    ) -> Result<B256, StatelessValidationError> {
        let mut removed_accounts = Vec::new();

        for (hashed_address, account) in state.accounts {
            // nonexisting accounts must be removed from the state
            let Some(account) = account else {
                removed_accounts.push(hashed_address);
                continue;
            };

            // apply storage changes before computing the storage root
            let storage_root = match state.storages.get(&hashed_address) {
                None => self.storage_trie_mut(hashed_address).unwrap().hash(),
                Some(storage) => {
                    let storage_trie = if storage.wiped {
                        self.clear_storage(hashed_address)
                    } else {
                        self.storage_trie_mut(hashed_address).unwrap()
                    };

                    // apply all state modifications
                    for (hashed_key, value) in &storage.storage {
                        if !value.is_zero() {
                            storage_trie.insert(*hashed_key, alloy_rlp::encode(value).into());
                        }
                    }
                    // removals must happen last, otherwise unresolved orphans might still exist
                    for (hashed_key, value) in &storage.storage {
                        if value.is_zero() {
                            storage_trie.remove(*hashed_key);
                        }
                    }

                    storage_trie.hash()
                }
            };

            // update/insert the account after all changes have been processed
            let account = TrieAccount {
                nonce: account.nonce,
                balance: account.balance,
                storage_root,
                code_hash: account.bytecode_hash.unwrap_or(KECCAK256_EMPTY),
            };
            self.state
                .insert(hashed_address, alloy_rlp::encode(account).into());
        }

        removed_accounts
            .iter()
            .for_each(|hashed_address| self.remove_account(hashed_address));

        Ok(self.state.hash())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_consensus::Header;
    use alloy_primitives::hex;
    use reth_primitives_traits::account::Account;

    #[test]
    fn test_sparse_state() {
        let state: Vec<Bytes> = {
            [
                Bytes::from(hex!("0xf869a0206aea581b220579a2b99819299dd32c7c28a420018ecb0bde93af007ad89a31b846f8440180a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a078c6cb5202685228bbcbfb992b1c4e116c7ec5ef11e25b8e92716cfc628ddd60")),
                Bytes::from(hex!("0xf869a037d65eaa92c6bc4c13a5ec45527f0c18ea8932588728769ec7aecfe6d9f32e42b846f8440180a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a0f57acd40259872606d76197ef052f3d35588dadf919ee1f0e3cb9b62d3f4b02c")),
                Bytes::from(hex!("0xf8b1a0c4b823e1deb537a6b4c41ecc9123e37753d61894f9dee7022b29c83088f69cfba00d1c2f6add00c6786d64a77d4136f71ef02f4a69307c77b663f32875ae8c7d9780a066a64e47bae97c0fccdc260c76b1c987c89560cb40e86ea17a1d5fd49e35bebe8080a039e4714d1eb6e1d5b21ca2bffd56333a7cd697596ff64317d1ae21ffd048e6ca808080808080a008be39f7c15cc06a7d863615397887281eadcbdb7907665d0683ca3c6383e6b0808080")),
                Bytes::from(hex!("0xf869a03f86c581c7d7b44eecbb92fd9e5867945ec1acdc0ea5bbabda21d17dddf06473b846f8440180a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a00345a365d2f4c5975b9f1599abe0a2ee76b7a3a731bc68781bd04c84e4858f50")),
                Bytes::from(hex!("0xf869a03d7dcb6a0ce5227c5379fc5b0e004561d7833b063355f69bfea3178f08fbaab4b846f8440180a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a09fb907ad9cb2872884a1e6839fcf89d229ef9b43df0511f58dbb26a1217ecb0d")),
                Bytes::from(hex!("0xf851808080a0de090f75dbe520ac527f21140ede3807a7dc416a0bae24c33dde9fe04300a08c808080808080808080a0f215e6bc9ca85972bc2488943dca80313a019f5eb569cc6ee3dc8c2af68734af808080")),
                Bytes::from(hex!("0x80")),
                Bytes::from(hex!("0xf851808080808080808080808080a031357c4a138624e300159fc631211a29d8373db4bdf59b80dad6e816593d0bcb8080a0b5790ff14363bee5d40c4a9fd9d6a515fc44683cc4d46666b4d9c775dded101780")),
                Bytes::from(hex!("0xf871a020601462093b5945d1676df093446790fd31b20e7b12a2e8e5e09d068109616bb84ef84c80880de0b6b3a7640000a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a0c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")),
                Bytes::from(hex!("0xf869a0209d57be05dd69371c4dd2e871bce6e9f4124236825bb612ee18a45e5675be51b846f8440180a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a06e49e66782037c0555897870e29fa5e552daf4719552131a0abce779daec0a5d"))
            ].to_vec()
        };

        let codes: Vec<Bytes> = {
            [
            Bytes::from(hex!("0x3373fffffffffffffffffffffffffffffffffffffffe14604d57602036146024575f5ffd5b5f35801560495762001fff810690815414603c575f5ffd5b62001fff01545f5260205ff35b5f5ffd5b62001fff42064281555f359062001fff015500")),
            Bytes::from(hex!("0x3373fffffffffffffffffffffffffffffffffffffffe14604657602036036042575f35600143038111604257611fff81430311604257611fff9006545f5260205ff35b5f5ffd5b5f35611fff60014303065500")),
            Bytes::from(hex!("0x3373fffffffffffffffffffffffffffffffffffffffe1460cb5760115f54807fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff146101f457600182026001905f5b5f82111560685781019083028483029004916001019190604d565b909390049250505036603814608857366101f457346101f4575f5260205ff35b34106101f457600154600101600155600354806003026004013381556001015f35815560010160203590553360601b5f5260385f601437604c5fa0600101600355005b6003546002548082038060101160df575060105b5f5b8181146101835782810160030260040181604c02815460601b8152601401816001015481526020019060020154807fffffffffffffffffffffffffffffffff00000000000000000000000000000000168252906010019060401c908160381c81600701538160301c81600601538160281c81600501538160201c81600401538160181c81600301538160101c81600201538160081c81600101535360010160e1565b910180921461019557906002556101a0565b90505f6002555f6003555b5f54807fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff14156101cd57505f5b6001546002828201116101e25750505f6101e8565b01600290035b5f555f600155604c025ff35b5f5ffd")),
            Bytes::from(hex!("0x3373fffffffffffffffffffffffffffffffffffffffe1460d35760115f54807fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff1461019a57600182026001905f5b5f82111560685781019083028483029004916001019190604d565b9093900492505050366060146088573661019a573461019a575f5260205ff35b341061019a57600154600101600155600354806004026004013381556001015f358155600101602035815560010160403590553360601b5f5260605f60143760745fa0600101600355005b6003546002548082038060021160e7575060025b5f5b8181146101295782810160040260040181607402815460601b815260140181600101548152602001816002015481526020019060030154905260010160e9565b910180921461013b5790600255610146565b90505f6002555f6003555b5f54807fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff141561017357505f5b6001546001828201116101885750505f61018e565b01600190035b5f555f6001556074025ff35b5f5ffd")),
            Bytes::from(hex!("0x366000600037600060003660006000600b610177f26000553d6001553d600060003e3d600020600255")),
            Bytes::from(hex!("0x"))
        ].to_vec()
        };

        let keys: Vec<Bytes> = {
            [
                Bytes::from(hex!("0xa94f5374fce5edbc8e2a8697c15331677e6ebf0b")),
                Bytes::from(hex!("0x0000f90827f1c53a10cb7a02335b175320002935")),
                Bytes::from(hex!(
                    "0x0000000000000000000000000000000000000000000000000000000000000000"
                )),
                Bytes::from(hex!("0x00000961ef480eb55e80d19ad83579a64c007002")),
                Bytes::from(hex!(
                    "0x0000000000000000000000000000000000000000000000000000000000000002"
                )),
                Bytes::from(hex!(
                    "0x0000000000000000000000000000000000000000000000000000000000000000"
                )),
                Bytes::from(hex!(
                    "0x0000000000000000000000000000000000000000000000000000000000000003"
                )),
                Bytes::from(hex!(
                    "0x0000000000000000000000000000000000000000000000000000000000000001"
                )),
                Bytes::from(hex!("0x000f3df6d732807ef1319fb7b8bb8522d0beac02")),
                Bytes::from(hex!(
                    "0x00000000000000000000000000000000000000000000000000000000000003e8"
                )),
                Bytes::from(hex!(
                    "0x00000000000000000000000000000000000000000000000000000000000023e7"
                )),
                Bytes::from(hex!("0x0000bbddc7ce488642fb579f8b00f3a590007251")),
                Bytes::from(hex!(
                    "0x0000000000000000000000000000000000000000000000000000000000000003"
                )),
                Bytes::from(hex!(
                    "0x0000000000000000000000000000000000000000000000000000000000000001"
                )),
                Bytes::from(hex!(
                    "0x0000000000000000000000000000000000000000000000000000000000000002"
                )),
                Bytes::from(hex!(
                    "0x0000000000000000000000000000000000000000000000000000000000000000"
                )),
                Bytes::from(hex!("0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba")),
                Bytes::from(hex!("0x0000000000000000000000000000000000001000")),
                Bytes::from(hex!(
                    "0x0000000000000000000000000000000000000000000000000000000000000002"
                )),
                Bytes::from(hex!(
                    "0x0000000000000000000000000000000000000000000000000000000000000000"
                )),
                Bytes::from(hex!(
                    "0x0000000000000000000000000000000000000000000000000000000000000001"
                )),
            ]
            .to_vec()
        };

        let headers: Vec<Bytes> = {
            [
            Bytes::from(hex!("0xf90257a00000000000000000000000000000000000000000000000000000000000000000a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347940000000000000000000000000000000000000000a05e5fc7fb30faa5cdc163023c4ce2dc8807601ec858dd2905738dad824d0a21cea056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421b901000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000080808402255100808000a0000000000000000000000000000000000000000000000000000000000000000088000000000000000007a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b4218080a00000000000000000000000000000000000000000000000000000000000000000a0e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"))
        ].to_vec()
        };

        let pre_state_root: B256 = alloy_rlp::decode_exact::<Header>(headers.get(0).unwrap())
            .unwrap()
            .state_root
            .clone();

        let ew = ExecutionWitness {
            state,
            codes,
            keys: keys.clone(),
            headers,
        };

        let trie = SimpleSparseState::new(&ew, pre_state_root);
        assert!(trie.is_ok(), "Error creating trie");
        let mut trie = trie.unwrap();
        // Verify root hash
        assert_eq!(trie.0.state.hash(), pre_state_root);
        assert_eq!(
            trie.0
                .calculate_state_root(HashedPostState::default())
                .unwrap(),
            pre_state_root
        );

        // Calculate post-state root. Change an account balance value and recalculate root hash.
        let mut accounts = B256Map::<Option<Account>>::default();
        let address = Address::from_slice(&hex!("0x00000961ef480eb55e80d19ad83579a64c007002"));
        let a = trie.0.account(address).unwrap().unwrap();
        accounts.insert(
            keccak256(address),
            Some(Account {
                nonce: a.nonce,
                balance: a.balance + U256::from(1),
                bytecode_hash: Some(a.code_hash.clone()),
            }),
        );
        let hashed_post_state = HashedPostState {
            accounts,
            storages: B256Map::default(),
        };

        println!(
            "{}",
            trie.0.calculate_state_root(hashed_post_state).unwrap()
        );
    }
}
