//! # Encode and Decode Predicates
//!
//! # Encoding
//! ## Predicate
//! | Field | Size (bytes) | Description |
//! | --- | --- | --- |
//! | number_of_nodes | 2 | The number of nodes in the predicate. |
//! | nodes | 35 * number_of_nodes | The nodes in the predicate. |
//! | number_of_edges | 2 | The number of edges in the predicate. |
//! | edges | 2 * number_of_edges | The edges in the predicate. |
//!
//! ## Node
//! | Field | Size (bytes) | Description |
//! | --- | --- | --- |
//! | edge_start | 2 | The index of the first edge in the edge list. |
//! | program_address | 32 | The address of the program. |
//! | reads | 1 | The type of state this program has access to. |
//!
//! ## Edge
//! | Field | Size (bytes) | Description |
//! | --- | --- | --- |
//! | edge | 2 | The index of the node that this edge points to. |
//!
//! ## Programs
//! | Field | Size (bytes) | Description |
//! | --- | --- | --- |
//! | number_of_programs | 2 | The number of programs in the set. |
//! | programs | variable | The programs in the set. |
//!
//! ## Program
//! | Field | Size (bytes) | Description |
//! | --- | --- | --- |
//! | program_length | 2 | The length of the program. |
//! | program | program_length | The program. |

use super::*;

#[cfg(test)]
mod tests;

const NODE_SIZE_BYTES: usize = 35;
const EDGE_SIZE_BYTES: usize = core::mem::size_of::<u16>();
const LEN_SIZE_BYTES: usize = core::mem::size_of::<u16>();

/// Errors that can occur when encoding a predicate.
#[derive(Debug, PartialEq)]
pub enum PredicateEncodeError {
    /// The predicate contains too many nodes.
    TooManyNodes,
    /// The predicate contains too many edges.
    TooManyEdges,
}

/// Errors that can occur when decoding a predicate.
#[derive(Debug, PartialEq)]
pub enum PredicateDecodeError {
    /// The bytes are too short to contain the number of nodes.
    BytesTooShort,
}

/// Errors that can occur when encoding a set of programs.
#[derive(Debug, PartialEq)]
pub enum ProgramsEncodeError {
    /// The set of programs is too large.
    TooLarge,
}

/// Errors that can occur when encoding a program.
#[derive(Debug, PartialEq)]
pub enum ProgramEncodeError {
    /// The program is too large.
    TooLarge,
}

/// Errors that can occur when decoding a program.
#[derive(Debug, PartialEq)]
pub enum ProgramDecodeError {
    /// The bytes are too short to contain the number of bytes.
    BytesTooShort,
}

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
                .chain(Some(node.reads as u8))
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
                        node[2..34].try_into().expect("safe due to chunks exact"),
                    ),
                    reads: Reads::from(node[34]),
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

/// Encode programs into bytes.
pub fn encode_programs(
    programs: &[Program],
) -> Result<impl Iterator<Item = u8> + '_, ProgramsEncodeError> {
    let len = if programs.len() <= Programs::MAX_PROGRAMS as usize {
        programs.len() as u16
    } else {
        return Err(ProgramsEncodeError::TooLarge);
    };
    if programs
        .iter()
        .any(|program| program.0.len() > Program::MAX_SIZE as usize)
    {
        return Err(ProgramsEncodeError::TooLarge);
    }
    let iter = len.to_be_bytes().into_iter().chain(
        programs
            .iter()
            .flat_map(|program| encode_program(program).into_iter())
            .flatten(),
    );
    Ok(iter)
}

/// Decode programs from bytes.
pub fn decode_programs(bytes: &[u8]) -> Result<Programs, ProgramDecodeError> {
    let Some(len) = bytes.get(..LEN_SIZE_BYTES).map(|x| {
        let mut arr = [0; LEN_SIZE_BYTES];
        arr.copy_from_slice(x);
        u16::from_be_bytes(arr)
    }) else {
        return Err(ProgramDecodeError::BytesTooShort);
    };
    let start = LEN_SIZE_BYTES;
    let mut programs = Vec::with_capacity(len as usize);

    let Some(mut bytes) = bytes.get(start..) else {
        return Err(ProgramDecodeError::BytesTooShort);
    };

    for _ in 0..len {
        let program = decode_program(bytes)?;
        let start = LEN_SIZE_BYTES + program.0.len();
        let Some(b) = bytes.get(start..) else {
            return Err(ProgramDecodeError::BytesTooShort);
        };
        bytes = b;
        programs.push(program);
    }
    Ok(Programs(programs))
}

/// Encode a program into bytes.
pub fn encode_program(
    program: &Program,
) -> Result<impl Iterator<Item = u8> + '_, ProgramEncodeError> {
    let len = if program.0.len() <= Program::MAX_SIZE as usize {
        program.0.len() as u16
    } else {
        return Err(ProgramEncodeError::TooLarge);
    };
    let iter = len
        .to_be_bytes()
        .into_iter()
        .chain(program.0.iter().copied());
    Ok(iter)
}

/// Decode a program from bytes.
pub fn decode_program(bytes: &[u8]) -> Result<Program, ProgramDecodeError> {
    let Some(len) = bytes.get(..LEN_SIZE_BYTES).map(|x| {
        let mut arr = [0; LEN_SIZE_BYTES];
        arr.copy_from_slice(x);
        u16::from_be_bytes(arr)
    }) else {
        return Err(ProgramDecodeError::BytesTooShort);
    };
    let start = LEN_SIZE_BYTES;
    let end = start + len as usize;
    let Some(program) = bytes.get(start..end) else {
        return Err(ProgramDecodeError::BytesTooShort);
    };
    let program = Program(program.to_vec());
    Ok(program)
}

impl Reads {
    fn from(byte: u8) -> Self {
        match byte % (Self::Post as u8 + 1) {
            0 => Self::Pre,
            1 => Self::Post,
            _ => unreachable!(),
        }
    }
}