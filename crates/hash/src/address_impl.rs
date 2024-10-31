use essential_types::{
    contract::Contract,
    predicate::{Predicate, Program},
    solution::Solution,
    Block, ContentAddress,
};

use crate::{hash, Address};

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

impl Address for Solution {
    fn content_address(&self) -> ContentAddress {
        ContentAddress(hash(self))
    }
}
