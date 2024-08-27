use essential_types::{
    contract::Contract, predicate::Predicate, solution::Solution, Block, ContentAddress,
};

use crate::Address;

impl Address for Block {
    fn content_address(&self) -> ContentAddress {
        crate::block_addr::from_block(self)
    }
}

impl Address for Predicate {}

impl Address for Contract {
    fn content_address(&self) -> ContentAddress {
        crate::contract_addr::from_contract(self)
    }
}

impl Address for Solution {}
