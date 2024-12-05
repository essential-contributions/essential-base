//! A small collection of helper functions to assist in the calculation of an
//! solution's content address.

use essential_types::{solution::SolutionSet, ContentAddress};

/// Determine the content address for the given `SolutionSet`.
///
/// If you have already calculated the content address for each `Solution` consider
/// using [`from_solution_addrs`] or [`from_solution_addrs_slice`].
pub fn from_set(set: &SolutionSet) -> ContentAddress {
    let solution_addrs = set.solutions.iter().map(crate::content_addr);
    from_solution_addrs(solution_addrs)
}

/// Given the content address for each `Solution` in the `SolutionSet`, produce the
/// solution set's content address.
///
/// This collects all yielded content addresses into a `Vec`, sorts them and then
/// hashes the result to produce the solution address.
///
/// If you have already collected the content address for each `Solution` into a
/// slice, consider [`from_solution_addrs_slice`].
pub fn from_solution_addrs(
    solution_addrs: impl IntoIterator<Item = ContentAddress>,
) -> ContentAddress {
    let mut solution_addrs: Vec<_> = solution_addrs.into_iter().collect();
    from_solution_addrs_slice(&mut solution_addrs)
}

/// Given the content address for each `Solution` in the `SolutionSet`, produce the
/// solution's content address.
///
/// This first sorts `solution_addrs` before producing the content address of the
/// slice, ensuring that the address maintains "set" semantics (i.e. the order
/// of its inner `Solution`s does not matter).
pub fn from_solution_addrs_slice(solution_addrs: &mut [ContentAddress]) -> ContentAddress {
    solution_addrs.sort();
    ContentAddress(crate::hash_bytes_iter(
        solution_addrs.iter().map(|addr| &addr.0[..]),
    ))
}
