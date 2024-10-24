use crate::StateRead as Op;
use bitflags::bitflags;

/// Flags to indicate the type of state read operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct StateReadFlags(u32);

bitflags! {
    impl StateReadFlags: u32 {
        /// No state read operations.
        const None = 0b00000000;
        /// [´Op::KeyRange´] state read operations´
        const Range = 0b00000001;
        /// [´Op::KeyRangeExtern´] state read operations´
        const RangeExtern = 0b00000011;
    }
}

/// Analyse the given operations to determine the type of state read operations.
///
/// [´Op::KeyRangeExtern´] has precedence over [´Op::KeyRange´].
pub fn analyse(ops: Vec<Op>) -> StateReadFlags {
    let mut key_range_exists = false;

    for op in &ops {
        match op {
            Op::KeyRangeExtern => return StateReadFlags::RangeExtern,
            Op::KeyRange => key_range_exists = true,
            _ => {}
        }
    }
    if key_range_exists {
        StateReadFlags::Range
    } else {
        StateReadFlags::None
    }
}

#[cfg(test)]
mod test {
    use super::{analyse, StateReadFlags};

    #[test]
    fn key_range() {
        let ops = vec![crate::Op::KeyRange];
        assert_eq!(analyse(ops), StateReadFlags::Range);
    }

    #[test]
    fn key_range_extern() {
        let ops = vec![crate::Op::KeyRangeExtern, crate::Op::KeyRange];
        let ops2 = vec![crate::Op::KeyRange, crate::Op::KeyRangeExtern];

        assert_eq!(analyse(ops), StateReadFlags::RangeExtern);
        assert_eq!(analyse(ops2), StateReadFlags::RangeExtern);
    }

    #[test]
    fn none() {
        let ops = vec![];
        assert_eq!(analyse(ops), StateReadFlags::None);
    }
}
