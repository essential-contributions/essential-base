//! `core::fmt` implementations and related items.

use crate::{serde::hash::BASE64, ContentAddress, Signature};
use base64::Engine;
use core::{fmt, str};

impl fmt::LowerHex for ContentAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
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

impl fmt::Display for ContentAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        crate::serde::hash::BASE64.encode(&self.0).fmt(f)
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let bytes: [u8; 65] = self.clone().into();
        crate::serde::hash::BASE64.encode(&bytes).fmt(f)
    }
}

impl str::FromStr for ContentAddress {
    type Err = base64::DecodeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vec = BASE64.decode(s)?;
        let len = vec.len();
        let bytes: [u8; 32] = vec
            .try_into()
            .map_err(|_| base64::DecodeError::InvalidLength(len))?;
        Ok(bytes.into())
    }
}

impl str::FromStr for Signature {
    type Err = base64::DecodeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vec = BASE64.decode(s)?;
        let len = vec.len();
        let bytes: [u8; 65] = vec
            .try_into()
            .map_err(|_| base64::DecodeError::InvalidLength(len))?;
        Ok(bytes.into())
    }
}
