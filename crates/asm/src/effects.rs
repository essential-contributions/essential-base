use crate::{Access, Op, StateRead};
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

#[cfg(test)]
mod test {
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
}
