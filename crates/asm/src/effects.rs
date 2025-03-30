use crate::{Access, Op, Stack, StateRead, ToOpcode};
use bitflags::bitflags;

/// Flags representing the set of effects caused by a given slice of operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Effects(u8);

bitflags! {
    impl Effects: u8 {
        /// Flag for [´StateRead::KeyRange´]
        const KeyRange = 1 << 0;
        /// Flag for [´StateRead::KeyRangeExtern´]
        const KeyRangeExtern = 1 << 1;
        /// Flag for [´Access::ThisAddress´]
        const ThisAddress = 1 << 2;
        /// Flag for [´Access::ThisContractAddress´]
        const ThisContractAddress = 1 << 3;
        /// Flag for [´StateRead::PostKeyRange´]
        const PostKeyRange = 1 << 4;
        /// Flag for [´StateRead::PostKeyRangeExtern´]
        const PostKeyRangeExtern = 1 << 5;
    }
}

/// Determine effects of the given program.
pub fn analyze(ops: &[Op]) -> Effects {
    let mut effects = Effects::empty();

    for op in ops {
        match op {
            Op::StateRead(StateRead::KeyRangeExtern) => effects |= Effects::KeyRangeExtern,
            Op::StateRead(StateRead::KeyRange) => effects |= Effects::KeyRange,
            Op::Access(Access::ThisAddress) => effects |= Effects::ThisAddress,
            Op::Access(Access::ThisContractAddress) => effects |= Effects::ThisContractAddress,
            _ => {}
        }

        // Short circuit if all flags are found.
        if effects == Effects::all() {
            break;
        }
    }
    effects
}

/// Analyze a slice of bytes to determine if it contains any of the effects.
///
/// This is a short-circuiting function that will return true if any of the effects
/// are found in the byte slice.
pub fn bytes_contains_any(bytes: &[u8], effects: Effects) -> bool {
    let krng_byte: u8 = Op::StateRead(StateRead::KeyRange).to_opcode().into();
    let krng_extern_byte: u8 = Op::StateRead(StateRead::KeyRangeExtern).to_opcode().into();
    let post_krng_byte: u8 = Op::StateRead(StateRead::PostKeyRange).to_opcode().into();
    let post_krng_extern_byte: u8 = Op::StateRead(StateRead::PostKeyRangeExtern)
        .to_opcode()
        .into();
    let this_address_byte: u8 = Op::Access(Access::ThisAddress).to_opcode().into();
    let this_contract_address_byte: u8 = Op::Access(Access::ThisContractAddress).to_opcode().into();

    let push: u8 = Op::Stack(Stack::Push(0)).to_opcode().into();

    let mut iter = bytes.iter();
    while let Some(byte) = iter.next() {
        match byte {
            b if *b == krng_byte && effects.contains(Effects::KeyRange) => return true,
            b if *b == krng_extern_byte && effects.contains(Effects::KeyRangeExtern) => {
                return true
            }
            b if *b == post_krng_byte && effects.contains(Effects::PostKeyRange) => return true,
            b if *b == post_krng_extern_byte && effects.contains(Effects::PostKeyRangeExtern) => {
                return true
            }
            b if *b == this_address_byte && effects.contains(Effects::ThisAddress) => return true,
            b if *b == this_contract_address_byte
                && effects.contains(Effects::ThisContractAddress) =>
            {
                return true
            }
            b if *b == push => {
                // Consume pushes arguments
                iter.by_ref().take(8).for_each(|_| ());
            }
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod test {
    use essential_types::convert::word_from_bytes;

    use crate::{effects::bytes_contains_any, ToOpcode};

    use super::{analyze, Access, Effects, Op, StateRead};

    #[test]
    fn none() {
        let ops = &[];
        assert_eq!(analyze(ops), Effects::empty());
    }

    #[test]
    fn key_range() {
        let ops = &[Op::StateRead(StateRead::KeyRange)];
        let effects = analyze(ops);
        assert!(effects.contains(Effects::KeyRange));
    }

    #[test]
    fn key_range_extern() {
        let ops = &[Op::StateRead(StateRead::KeyRangeExtern)];
        let effects = analyze(ops);
        assert!(effects.contains(Effects::KeyRangeExtern));
    }

    #[test]
    fn this_address() {
        let ops = &[Op::Access(Access::ThisAddress)];
        let effects = analyze(ops);
        assert!(effects.contains(Effects::ThisAddress));
    }

    #[test]
    fn this_contract_address() {
        let ops = &[Op::Access(Access::ThisContractAddress)];
        let effects = analyze(ops);
        assert!(effects.contains(Effects::ThisContractAddress));
    }

    #[test]
    fn all_effects() {
        let ops = &[
            Op::StateRead(StateRead::KeyRange),
            Op::StateRead(StateRead::KeyRangeExtern),
            Op::Access(Access::ThisAddress),
            Op::Access(Access::ThisContractAddress),
        ];
        let effects = analyze(ops);
        assert!(effects.contains(Effects::KeyRange));
        assert!(effects.contains(Effects::KeyRangeExtern));
        assert!(effects.contains(Effects::ThisAddress));
        assert!(effects.contains(Effects::ThisContractAddress));
    }

    #[test]
    fn test_bytes_contains_any() {
        use crate::short::*;
        let to_bytes = |ops: &[Op]| crate::to_bytes(ops.iter().copied()).collect::<Vec<u8>>();

        // No effects found
        let effects = Effects::all();
        assert!(!bytes_contains_any(&to_bytes(&[POP, POP, POP]), effects));

        // Contains different effects
        let effects = Effects::KeyRange | Effects::KeyRangeExtern;
        assert!(!bytes_contains_any(&to_bytes(&[PKRNG, PKREX]), effects));

        // Push contains key opcode
        let key: u8 = PKRNG.to_opcode().into();
        let bytes = [key, 0, 0, 0, 0, 0, 0, 0];
        let word = word_from_bytes(bytes);
        let effects = Effects::PostKeyRange;
        assert!(!bytes_contains_any(&to_bytes(&[PUSH(word)]), effects));

        // Push doesn't prevent detection
        let effects = Effects::KeyRange;
        assert!(bytes_contains_any(&to_bytes(&[PUSH(word), KRNG]), effects));

        // Key range
        let effects = Effects::KeyRange;
        assert!(bytes_contains_any(&to_bytes(&[KRNG]), effects));

        // Key range extern
        let effects = Effects::KeyRangeExtern;
        assert!(bytes_contains_any(&to_bytes(&[KREX]), effects));

        // Post key range
        let effects = Effects::PostKeyRange;
        assert!(bytes_contains_any(&to_bytes(&[PKRNG]), effects));

        // Post key range extern
        let effects = Effects::PostKeyRangeExtern;
        assert!(bytes_contains_any(&to_bytes(&[PKREX]), effects));

        // This address
        let effects = Effects::ThisAddress;
        assert!(bytes_contains_any(&to_bytes(&[THIS]), effects));

        // This contract address
        let effects = Effects::ThisContractAddress;
        assert!(bytes_contains_any(&to_bytes(&[THISC]), effects));

        // Empty
        let effects = Effects::empty();
        assert!(!bytes_contains_any(&[], effects));

        let effects = Effects::KeyRange;
        assert!(!bytes_contains_any(&[], effects));
    }
}
