use crate::Address;
use essential_types::{
    contract::Contract,
    convert::bytes_from_word,
    predicate::Predicate,
    solution::{Solution, SolutionData},
    Block, ContentAddress, Word,
};
use sha2::Digest;

impl Address for Block {
    fn content_address(&self) -> ContentAddress {
        crate::block_addr::from_block(self)
    }
}

impl Address for Predicate {
    fn content_address(&self) -> ContentAddress {
        let Ok(header) = self.encoded_header() else {
            // Invalid predicates can't be hashed.
            return ContentAddress([0; 32]);
        };
        let mut hasher = <sha2::Sha256 as Digest>::new();
        hasher.update(header.fixed_size_header.0);
        hasher.update(header.lens);
        for item in self.programs() {
            hasher.update(item);
        }
        ContentAddress(hasher.finalize().into())
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
        let mut hasher = <sha2::Sha256 as Digest>::new();

        // Include the predicate address.
        hasher.update(self.predicate_to_solve.contract.0);
        hasher.update(self.predicate_to_solve.predicate.0);

        // Decision variables must be in a particular order, required by the predicate
        // that is being solved. Thus, we hash them in their existing order.
        hasher.update(bytes_from_word(self.decision_variables.len() as Word));
        for value in &self.decision_variables {
            crate::hash_len_then_words(value, &mut hasher);
        }

        // State mutations are a set. In order to ensure the same CA is produced regardless
        // of the ordering of the state mutations, we hash the mutations in order of `Key`.
        hasher.update(bytes_from_word(self.state_mutations.len() as Word));
        let mut ixs: Vec<_> = (0..self.state_mutations.len()).collect();
        ixs.sort_by_key(|&ix| &self.state_mutations[ix].key);
        ixs.iter().for_each(|&ix| {
            let pair = &self.state_mutations[ix];
            crate::hash_len_then_words(&pair.key, &mut hasher);
            crate::hash_len_then_words(&pair.value, &mut hasher);
        });

        ContentAddress(hasher.finalize().into())
    }
}

impl Address for Solution {
    fn content_address(&self) -> ContentAddress {
        crate::solution_addr::from_solution(self)
    }
}
