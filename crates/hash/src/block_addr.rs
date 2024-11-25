//! A small collection of helper functions to assist in the calculation of a
//! block's content address.
//!
//! Note that it is possible this hash will change in the future
//! if merkle proofs or a header type is introduced.

use essential_types::{Block, ContentAddress};

/// Shorthand for the common case of producing a block's content address
/// from a [`Block`].
///
/// *Note:* this also hashes each solution set.
/// If you already have the content address for each solution set, consider
/// using [`from_block_and_solution_set_addrs`] or [`from_block_and_solution_set_addrs_slice`].
pub fn from_block(block: &Block) -> ContentAddress {
    let solution_addrs = block.solution_sets.iter().map(crate::content_addr);
    from_block_and_solution_set_addrs(block, solution_addrs)
}

/// Given the content address for each solution set in the block, produce the
/// block's content address.
///
/// *Warning:* the caller **must** ensure that the order of the solution sets
/// matches the order of the solution sets in the block.
/// Otherwise the content address will be different then the one calculated
/// for the [`Block`].
pub fn from_block_and_solution_set_addrs(
    block: &Block,
    solution_set_addrs: impl IntoIterator<Item = ContentAddress>,
) -> ContentAddress {
    let solution_set_addrs: Vec<ContentAddress> = solution_set_addrs.into_iter().collect();
    from_block_and_solution_set_addrs_slice(block, &solution_set_addrs)
}

/// Given the content address for each solution set in the block, produce the
/// block's content address.
///
/// *Warning:* the caller **must** ensure that the order of the solution sets
/// matches the order of the solution sets in the block.
/// Otherwise the content address will be different then the one calculated
/// for the [`Block`].
pub fn from_block_and_solution_set_addrs_slice(
    block: &Block,
    solution_set_addrs: &[ContentAddress],
) -> ContentAddress {
    let Block {
        number,
        timestamp,
        solution_sets: _,
    } = block;
    let header = (number, timestamp, solution_set_addrs);
    ContentAddress(crate::hash(&header))
}
