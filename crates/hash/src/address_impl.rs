use crate::Address;
use essential_types::{
    contract::Contract,
    predicate::{Predicate, Program},
    solution::{Solution, SolutionData},
    Block, ContentAddress,
};

impl Address for Block {
    fn content_address(&self) -> ContentAddress {
        crate::block_addr::from_block(self)
    }
}

impl Address for Predicate {
    fn content_address(&self) -> ContentAddress {
        let Ok(bytes) = self.encode() else {
            // Invalid predicates can't be hashed.
            return ContentAddress([0; 32]);
        };
        let bytes: Vec<_> = bytes.collect();
        ContentAddress(crate::hash_bytes(&bytes))
    }
}

impl Address for Program {
    fn content_address(&self) -> ContentAddress {
        ContentAddress(crate::hash_bytes(&self.0))
    }
}

impl Address for Contract {
    fn content_address(&self) -> ContentAddress {
        crate::contract_addr::from_contract(self)
    }
}

/// Hash the [`SolutionData`] in a manner that treats `state_mutations` like a set.
///
/// Hashing occurs as follows:
///
/// - predicate_to_solve.contract.0
impl Address for SolutionData {
    fn content_address(&self) -> ContentAddress {
        ContentAddress(crate::hash(self))
    }
}

impl Address for Solution {
    fn content_address(&self) -> ContentAddress {
        crate::solution_addr::from_solution(self)
    }
}
