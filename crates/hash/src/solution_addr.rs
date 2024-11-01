//! A small collection of helper functions to assist in the calculation of an
//! solution's content address.

use essential_types::{solution::Solution, ContentAddress};

/// Shorthand for the common case of producing a solution address from an
/// iterator yielding references to [`SolutionData`]s.
///
/// If you have already calculated the content address for each `SolutionData` consider
/// using [`from_data_addrs`] or [`from_data_addrs_slice`].
pub fn from_solution(solution: &Solution) -> ContentAddress {
    let data_addrs = solution.data.iter().map(crate::content_addr);
    from_data_addrs(data_addrs)
}

/// Given the content address for each `SolutionData` in the `Solution`, produce the
/// solution's content address.
///
/// This collects all yielded content addresses into a `Vec`, sorts them and then
/// hashes the result to produce the solution address.
///
/// If you have already collected the content address for each `SolutionData` into a
/// slice, consider [`from_data_addrs_slice`].
pub fn from_data_addrs(data_addrs: impl IntoIterator<Item = ContentAddress>) -> ContentAddress {
    let mut data_addrs: Vec<_> = data_addrs.into_iter().collect();
    from_data_addrs_slice(&mut data_addrs)
}

/// Given the content address for each `SolutionData` in the `Solution`, produce the
/// solution's content address.
///
/// This first sorts `data_addrs` before producing the content address of the
/// slice, ensuring that the address maintains "solution" semantics (i.e. the order
/// of its inner `SolutionData` does not matter).
pub fn from_data_addrs_slice(data_addrs: &mut [ContentAddress]) -> ContentAddress {
    data_addrs.sort();
    ContentAddress(crate::hash_bytes_iter(
        data_addrs.iter().map(|addr| &addr.0[..]),
    ))
}
