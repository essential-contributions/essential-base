//! `core::fmt` implementations and related items.

use crate::{
    predicate::{PredicateDecodeError, PredicateEncodeError},
    solution::decode::MutationDecodeError,
    ContentAddress, PredicateAddress, Signature,
};
use core::{fmt, str};

impl fmt::LowerHex for ContentAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::LowerHex for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        write!(f, "{:02x}", self.1)?;
        Ok(())
    }
}

impl fmt::UpperHex for ContentAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02X}")?;
        }
        Ok(())
    }
}

impl fmt::UpperHex for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02X}")?;
        }
        write!(f, "{:02X}", self.1)?;
        Ok(())
    }
}

impl fmt::Debug for ContentAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for ContentAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        hex::encode_upper(self.0).fmt(f)
    }
}

impl fmt::Display for PredicateAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.contract, self.predicate)
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let bytes: [u8; 65] = self.clone().into();
        hex::encode_upper(bytes).fmt(f)
    }
}

impl fmt::Display for PredicateDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PredicateDecodeError::BytesTooShort => "bytes too short",
            }
        )
    }
}

impl fmt::Display for PredicateEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PredicateEncodeError::TooManyNodes => "too many nodes",
                PredicateEncodeError::TooManyEdges => "too many edges",
            }
        )
    }
}

impl fmt::Display for MutationDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MutationDecodeError::WordsTooShort =>
                    "bytes too short for lengths given for key or value",
                MutationDecodeError::NegativeKeyLength => "negative key length",
                MutationDecodeError::NegativeValueLength => "negative value length",
            }
        )
    }
}

impl str::FromStr for ContentAddress {
    type Err = hex::FromHexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vec = hex::decode(s)?;
        let bytes: [u8; 32] = vec
            .try_into()
            .map_err(|_| hex::FromHexError::InvalidStringLength)?;
        Ok(bytes.into())
    }
}

impl str::FromStr for Signature {
    type Err = hex::FromHexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vec = hex::decode(s)?;
        let bytes: [u8; 65] = vec
            .try_into()
            .map_err(|_| hex::FromHexError::InvalidStringLength)?;
        Ok(bytes.into())
    }
}
