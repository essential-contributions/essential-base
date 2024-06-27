//! # Contract
//!
//! Types needed to represent an contract.

use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

use crate::{predicate::Predicate, Hash, Signature};

/// A contract of predicates whose content address has been signed.
///
/// For a shorthand constructor, see the downstream
/// `essential_sign::contract::sign` function.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SignedContract {
    /// The contract of predicates whose content address has been signed.
    pub contract: Contract,
    /// A signature over the contract's content address.
    ///
    /// This signature must be produced by signing the contract's
    /// [`ContentAddress`][crate::ContentAddress]. The contract's
    /// content address can be produced using one of the downstream
    /// `essential_hash::contract_addr` functions.
    pub signature: Signature,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// A contract of predicates.
pub struct Contract {
    /// The contract of predicates.
    pub predicates: Vec<Predicate>,
    /// The salt used to make the contract unique.
    pub salt: Hash,
}

impl Contract {
    /// Create a new contract with the given predicates but no salt.
    pub fn without_salt(predicates: Vec<Predicate>) -> Self {
        Self {
            predicates,
            ..Default::default()
        }
    }

    /// Turn the contract into its inner parts.
    pub fn into_inner(self) -> (Vec<Predicate>, Hash) {
        (self.predicates, self.salt)
    }
}

impl From<Vec<Predicate>> for Contract {
    fn from(predicates: Vec<Predicate>) -> Self {
        Self {
            predicates,
            ..Default::default()
        }
    }
}

impl From<Contract> for Vec<Predicate> {
    fn from(contract: Contract) -> Self {
        contract.predicates
    }
}

impl AsRef<[Predicate]> for Contract {
    fn as_ref(&self) -> &[Predicate] {
        &self.predicates
    }
}

impl Deref for Contract {
    type Target = Vec<Predicate>;

    fn deref(&self) -> &Self::Target {
        &self.predicates
    }
}

impl DerefMut for Contract {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.predicates
    }
}
