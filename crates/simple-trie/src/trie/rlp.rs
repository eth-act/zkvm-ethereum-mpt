//! Implementation of a trie node rlp decoding.
//! Based on the implementation in the ` mpt ` module of this crate.
use crate::trie::TrieNode::{Branch, Digest, Leaf};
use super::nodes::{BranchNode, BranchNodeChildrenArray, DigestNode, LeafNode, TrieNode};
use alloy_primitives::{B256, Bytes};
use alloy_rlp::{Decodable, EMPTY_STRING_CODE, Header, PayloadView};
use alloy_trie::Nibbles;

impl TrieNode {
    pub(super) fn decode(rlp_rep: &mut &[u8]) -> Result<Option<Self>, alloy_rlp::Error> {
        match Header::decode_raw(rlp_rep)? {
            PayloadView::String(payload) => {
                if payload.is_empty() {
                    Ok(None)
                } else if payload.len() == 32 {
                    Ok(Some(Digest(DigestNode {
                        value: B256::from_slice(payload),
                        hash: None,
                        path: Nibbles::default(),
                    })))
                } else {
                    Err(alloy_rlp::Error::Custom("MPT: Invalid RLP string length"))
                }
            }
            PayloadView::List(list) => {
                if list.len() == 17 {
                    let mut children = BranchNodeChildrenArray::new();
                    for (idx, element) in list[..16].iter().enumerate() {
                        if *element != &[EMPTY_STRING_CODE] {
                            let mut element_ref = element.as_ref();
                            children.insert(idx, Box::new(
                                TrieNode::decode(&mut element_ref)?
                                    .expect("MPT: Unable to decode branch child node."),
                            ));
                        }
                    }
                    if list[16] != &[EMPTY_STRING_CODE] {
                        return Err(alloy_rlp::Error::Custom(
                            "MPT: Value in a branch node.",
                        ));
                    }
                    Ok(Some(Branch(BranchNode {
                        children,
                        hash: None,
                        path: Nibbles::default(),
                    })))
                } else if list.len() == 2 {
                    let [encoded_path, value] = list.as_slice() else {
                        unreachable!()
                    };
                    let mut encoded_path_ref = encoded_path.as_ref();
                    let (path, is_leaf) = decode_path(&mut encoded_path_ref)?;
                    if is_leaf {
                        let mut value_ref = value.as_ref();
                        Ok(Some(Leaf(LeafNode {
                            path,
                            value: Bytes::decode(&mut value_ref)?,
                            hash: None,
                        })))
                    } else {
                        let mut value_ref = value.as_ref();
                        let mut node = TrieNode::decode(&mut value_ref)?
                            .expect("MPT: Empty node in extension.");
                        match &mut node {
                            Branch(branch) => branch.path = path,
                            Digest(digest) => digest.path = path,
                            _ => {
                                return Err(alloy_rlp::Error::Custom(
                                    "MPT: Invalid extension node.",
                                ));
                            }
                        }
                        Ok(Some(node))
                    }
                } else {
                    Err(alloy_rlp::Error::Custom("MPT: Invalid RLP list length"))
                }
            }
        }
    }
}

#[inline]
fn decode_path(buf: &mut &[u8]) -> alloy_rlp::Result<(Nibbles, bool)> {
    let path = Nibbles::unpack(Header::decode_bytes(buf, false)?);
    if path.len() < 2 {
        return Err(alloy_rlp::Error::InputTooShort);
    }
    let (is_leaf, odd_nibbles) = match path.at(0) {
        0b0000 => (false, false),
        0b0001 => (false, true),
        0b0010 => (true, false),
        0b0011 => (true, true),
        _ => return Err(alloy_rlp::Error::Custom("node is not an extension or leaf")),
    };
    let path = if odd_nibbles {
        path.slice(1..)
    } else {
        path.slice(2..)
    };
    Ok((path, is_leaf))
}

// Encodes list header for known payload length. Reserves memory.
#[inline]
pub(super) fn encode_list_header(payload_length: usize) -> Vec<u8> {
    debug_assert!(payload_length > 1);
    let header = Header {
        list: true,
        payload_length,
    };
    let mut out = Vec::with_capacity(header.length() + payload_length);
    header.encode(&mut out);
    out
}
