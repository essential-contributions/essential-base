//! A small collection of helper functions to assist in the calculation of a
//! block's content address.
//!
//! Note that it is possible this hash will change in the future
//! if merkle proofs or a header type is introduced.

use essential_types::{Block, ContentAddress};

/// Shorthand for the common case of producing a block's content address
/// from a [`Block`].
///
/// *Note:* this also hashes each solution.
/// If you already have the content address for each solution consider
/// using [`from_block_and_solutions_addrs`] or [`from_block_and_solutions_addrs_slice`].
pub fn from_block(block: &Block) -> ContentAddress {
    let solution_addrs = block.solutions.iter().map(crate::content_addr);
    from_block_and_solutions_addrs(block, solution_addrs)
}

/// Given the content address for each solution in the block, produce the
/// block's content address.
///
/// *Warning:* the caller **must** ensure that the order of the solutions
/// matches the order of the solutions in the block.
/// Otherwise the content address will be different then the one calculated
/// for the [`Block`].
pub fn from_block_and_solutions_addrs(
    block: &Block,
    solution_addrs: impl IntoIterator<Item = ContentAddress>,
) -> ContentAddress {
    let solution_addrs: Vec<ContentAddress> = solution_addrs.into_iter().collect();
    from_block_and_solutions_addrs_slice(block, &solution_addrs)
}

/// Given the content address for each solution in the block, produce the
/// block's content address.
///
/// *Warning:* the caller **must** ensure that the order of the solutions
/// matches the order of the solutions in the block.
/// Otherwise the content address will be different then the one calculated
/// for the [`Block`].
pub fn from_block_and_solutions_addrs_slice(
    block: &Block,
    solution_addrs: &[ContentAddress],
) -> ContentAddress {
    let Block {
        number,
        timestamp,
        solutions: _,
    } = block;
    let header = (number, timestamp, solution_addrs);
    ContentAddress(crate::hash(&header))
}
