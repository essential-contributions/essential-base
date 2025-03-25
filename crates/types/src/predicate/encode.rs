//! # Encode and Decode Predicates
//!
//! # Encoding
//! ## Predicate
//! | Field | Size (bytes) | Description |
//! | --- | --- | --- |
//! | number_of_nodes | 2 | The number of nodes in the predicate. |
//! | nodes | 34 * number_of_nodes | The nodes in the predicate. |
//! | number_of_edges | 2 | The number of edges in the predicate. |
//! | edges | 2 * number_of_edges | The edges in the predicate. |
//!
//! ## Node
//! | Field | Size (bytes) | Description |
//! | --- | --- | --- |
//! | edge_start | 2 | The index of the first edge in the edge list. |
//! | program_address | 32 | The address of the program. |
//!
//! ## Edge
//! | Field | Size (bytes) | Description |
//! | --- | --- | --- |
//! | edge | 2 | The index of the node that this edge points to. |

use super::*;

#[cfg(test)]
mod tests;

const NODE_SIZE_BYTES: usize = 34;
const EDGE_SIZE_BYTES: usize = core::mem::size_of::<u16>();
const LEN_SIZE_BYTES: usize = core::mem::size_of::<u16>();

/// Errors that can occur when decoding a predicate.
#[derive(Debug, PartialEq)]
pub enum PredicateDecodeError {
    /// The bytes are too short to contain the number of nodes.
    BytesTooShort,
}

/// Errors that can occur when encoding a predicate.
#[derive(Debug, PartialEq)]
pub enum PredicateEncodeError {
    /// The predicate contains too many nodes.
    TooManyNodes,
    /// The predicate contains too many edges.
    TooManyEdges,
}

impl std::error::Error for PredicateDecodeError {}

impl std::error::Error for PredicateEncodeError {}

/// Encode a predicate into bytes.
pub fn encode_predicate(
    predicate: &Predicate,
) -> Result<impl Iterator<Item = u8> + '_, PredicateEncodeError> {
    let num_nodes = if predicate.nodes.len() <= Predicate::MAX_NODES as usize {
        predicate.nodes.len() as u16
    } else {
        return Err(PredicateEncodeError::TooManyNodes);
    };
    let num_edges = if predicate.edges.len() <= Predicate::MAX_EDGES as usize {
        predicate.edges.len() as u16
    } else {
        return Err(PredicateEncodeError::TooManyEdges);
    };
    let iter = num_nodes
        .to_be_bytes()
        .into_iter()
        .chain(predicate.nodes.iter().flat_map(|node| {
            node.edge_start
                .to_be_bytes()
                .into_iter()
                .chain(node.program_address.0.iter().copied())
        }))
        .chain(num_edges.to_be_bytes())
        .chain(predicate.edges.iter().flat_map(|edge| edge.to_be_bytes()));
    Ok(iter)
}

/// The size of the encoded predicate.
pub fn predicate_encoded_size(predicate: &Predicate) -> usize {
    predicate.nodes.len() * NODE_SIZE_BYTES + predicate.edges.len() * EDGE_SIZE_BYTES + 2
}

/// Decode a predicate from bytes.
pub fn decode_predicate(bytes: &[u8]) -> Result<Predicate, PredicateDecodeError> {
    let Some(num_nodes) = bytes.get(..LEN_SIZE_BYTES).map(|x| {
        let mut arr = [0; LEN_SIZE_BYTES];
        arr.copy_from_slice(x);
        u16::from_be_bytes(arr)
    }) else {
        return Err(PredicateDecodeError::BytesTooShort);
    };

    let nodes: Vec<_> =
        match bytes.get(LEN_SIZE_BYTES..(LEN_SIZE_BYTES + num_nodes as usize * NODE_SIZE_BYTES)) {
            Some(bytes) => bytes
                .chunks_exact(NODE_SIZE_BYTES)
                .take(num_nodes as usize)
                .map(|node| Node {
                    edge_start: u16::from_be_bytes(
                        node[..2].try_into().expect("safe due to chunks exact"),
                    ),
                    program_address: ContentAddress(
                        node[2..].try_into().expect("safe due to chunks exact"),
                    ),
                })
                .collect(),
            None => return Err(PredicateDecodeError::BytesTooShort),
        };

    let num_edges_pos = num_nodes as usize * NODE_SIZE_BYTES + LEN_SIZE_BYTES;
    let Some(num_edges) = bytes.get(num_edges_pos..(num_edges_pos + 2)).map(|x| {
        let mut arr = [0; 2];
        arr.copy_from_slice(x);
        u16::from_be_bytes(arr)
    }) else {
        return Err(PredicateDecodeError::BytesTooShort);
    };

    let edges_start = num_edges_pos + LEN_SIZE_BYTES;
    let edges: Vec<_> =
        match bytes.get(edges_start..(edges_start + num_edges as usize * EDGE_SIZE_BYTES)) {
            Some(bytes) => bytes
                .chunks_exact(EDGE_SIZE_BYTES)
                .map(|edge| {
                    let mut arr = [0; 2];
                    arr.copy_from_slice(edge);
                    u16::from_be_bytes(arr)
                })
                .collect(),
            None => return Err(PredicateDecodeError::BytesTooShort),
        };
    Ok(Predicate { nodes, edges })
}
