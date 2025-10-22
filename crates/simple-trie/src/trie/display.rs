//! Simple printing implementation of an MPT.
use crate::trie::TrieNode::{Branch, Digest, Leaf};
use crate::trie::{Trie, TrieNode};
use std::fmt::Display;

impl Display for Trie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.root.is_none() {
            return write!(f, "Trie {{ EMPTY }}");
        }

        fn fmt_node(
            f: &mut std::fmt::Formatter<'_>,
            node: &TrieNode,
            indent: usize,
        ) -> std::fmt::Result {
            write!(f, "{}", " ".repeat(indent))?;
            match node {
                Branch(branch) => {
                    write!(f, "Branch {:?}", branch.path.to_vec())?;
                    for child in branch.children.iter() {
                        if child.is_none() {
                            write!(f, "\n{}None", " ".repeat(indent + 4))?;
                        } else {
                            write!(f, "\n")?;
                            fmt_node(f, child.as_ref().unwrap(), indent + 4)?;
                        }
                    }
                    Ok(())
                }
                Leaf(leaf) => write!(
                    f,
                    "Leaf {{ path: {:?}, value: {:?} }}",
                    leaf.path.to_vec(),
                    leaf.value
                ),
                Digest(digest) => write!(
                    f,
                    "Digest {{ path: {:?}, digest: {:?} }}",
                    digest.path, digest.value
                ),
            }
        }

        fmt_node(f, &self.root.as_ref().unwrap(), 0)
    }
}
