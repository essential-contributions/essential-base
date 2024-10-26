use crate::StateRead as StateReadOp;
use bitflags::bitflags;
use essential_constraint_asm::{Access, Constraint as ConstraintOp};

/// Flags to indicate the effects of state read operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Effects(u8);

bitflags! {
    impl Effects: u8 {
        /// Flag for [´StateReadOp::KeyRange´]
        const Range = 1 << 0;
        /// Flag for [´StateReadOp::KeyRangeExtern´]
        const RangeExtern = 1 << 1;
        /// Flag for [´StateReadOp::Constraint::Access::ThisAddress´]
        const ThisAddress = 1 << 2;
        /// Flag for [´StateReadOp::Constraint::Access::ThisContractAddress´]
        const ThisContractAddress = 1 << 3;
    }
}

/// Determine effects of the given state read program.
pub fn determine_effects(ops: &[StateReadOp]) -> Effects {
    let mut effects = Effects::empty();

    for op in ops {
        match op {
            StateReadOp::KeyRangeExtern => effects |= Effects::RangeExtern,
            StateReadOp::KeyRange => effects |= Effects::Range,
            StateReadOp::Constraint(ConstraintOp::Access(Access::ThisAddress)) => {
                effects |= Effects::ThisAddress
            }
            StateReadOp::Constraint(ConstraintOp::Access(Access::ThisContractAddress)) => {
                effects |= Effects::ThisContractAddress
            }
            _ => {}
        }
    }
    effects
}

#[cfg(test)]
mod test {
    use super::{determine_effects, Access, ConstraintOp, Effects, StateReadOp};

    #[test]
    fn none() {
        let ops = &[];
        assert_eq!(determine_effects(ops), Effects::empty());
    }

    #[test]
    fn key_range() {
        let ops = &[StateReadOp::KeyRange];
        let effects = determine_effects(ops);
        assert!(effects.contains(Effects::Range));
    }

    #[test]
    fn key_range_extern() {
        let ops = &[StateReadOp::KeyRangeExtern];
        let effects = determine_effects(ops);
        assert!(effects.contains(Effects::RangeExtern));
    }

    #[test]
    fn this_address() {
        let ops = &[StateReadOp::Constraint(ConstraintOp::Access(
            Access::ThisAddress,
        ))];
        let effects = determine_effects(ops);
        assert!(effects.contains(Effects::ThisAddress));
    }

    #[test]
    fn this_contract_address() {
        let ops = &[StateReadOp::Constraint(ConstraintOp::Access(
            Access::ThisContractAddress,
        ))];
        let effects = determine_effects(ops);
        assert!(effects.contains(Effects::ThisContractAddress));
    }

    #[test]
    fn all_effects() {
        let ops = &[
            StateReadOp::KeyRange,
            StateReadOp::KeyRangeExtern,
            StateReadOp::Constraint(ConstraintOp::Access(Access::ThisAddress)),
            StateReadOp::Constraint(ConstraintOp::Access(Access::ThisContractAddress)),
        ];
        let effects = determine_effects(ops);
        assert!(effects.contains(Effects::Range));
        assert!(effects.contains(Effects::RangeExtern));
        assert!(effects.contains(Effects::ThisAddress));
        assert!(effects.contains(Effects::ThisContractAddress));
    }
}
