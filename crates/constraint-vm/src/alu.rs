//! ALU operation implementations.

use crate::{asm::Word, error::AluError, OpResult};

#[cfg(test)]
mod shifts;

pub(crate) fn add(a: Word, b: Word) -> OpResult<Word> {
    a.checked_add(b).ok_or(AluError::Overflow.into())
}

pub(crate) fn sub(a: Word, b: Word) -> OpResult<Word> {
    a.checked_sub(b).ok_or(AluError::Underflow.into())
}

pub(crate) fn mul(a: Word, b: Word) -> OpResult<Word> {
    a.checked_mul(b).ok_or(AluError::Overflow.into())
}

pub(crate) fn div(a: Word, b: Word) -> OpResult<Word> {
    a.checked_div(b).ok_or(AluError::DivideByZero.into())
}

pub(crate) fn mod_(a: Word, b: Word) -> OpResult<Word> {
    a.checked_rem(b).ok_or(AluError::DivideByZero.into())
}

pub(crate) fn shl(a: Word, b: Word) -> OpResult<Word> {
    check_shift_bounds(b)?;
    Ok(a << b)
}

pub(crate) fn shr(a: Word, b: Word) -> OpResult<Word> {
    check_shift_bounds(b)?;
    // casts are safe and turn this into a logical shift
    Ok(((a as u64) >> b) as Word)
}

pub(crate) fn arithmetic_shr(a: Word, b: Word) -> OpResult<Word> {
    check_shift_bounds(b)?;
    Ok(a >> b)
}

const BITS_IN_WORD: Word = core::mem::size_of::<Word>() as Word * 8;

#[inline]
fn check_shift_bounds(b: Word) -> OpResult<()> {
    let bounds = 0..BITS_IN_WORD;
    if !bounds.contains(&b) {
        return Err(AluError::Overflow.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        asm::{Alu, Pred, Stack, Word},
        error::{AluError, ConstraintError, OpError},
        eval_ops,
        test_util::*,
    };

    #[test]
    fn eval_6_mul_7_eq_42() {
        let ops = &[
            Stack::Push(6).into(),
            Stack::Push(7).into(),
            Alu::Mul.into(),
            Stack::Push(42).into(),
            Pred::Eq.into(),
        ];
        eval_ops(ops, *test_access()).unwrap();
    }

    #[test]
    fn eval_42_div_6_eq_7() {
        let ops = &[
            Stack::Push(42).into(),
            Stack::Push(7).into(),
            Alu::Div.into(),
            Stack::Push(6).into(),
            Pred::Eq.into(),
        ];
        eval_ops(ops, *test_access()).unwrap();
    }

    #[test]
    fn eval_divide_by_zero() {
        let ops = &[
            Stack::Push(42).into(),
            Stack::Push(0).into(),
            Alu::Div.into(),
        ];
        match eval_ops(ops, *test_access()) {
            Err(ConstraintError::Op(_, OpError::Alu(AluError::DivideByZero))) => (),
            _ => panic!("expected ALU divide-by-zero error"),
        }
    }

    #[test]
    fn eval_add_overflow() {
        let ops = &[
            Stack::Push(Word::MAX).into(),
            Stack::Push(1).into(),
            Alu::Add.into(),
        ];
        match eval_ops(ops, *test_access()) {
            Err(ConstraintError::Op(_, OpError::Alu(AluError::Overflow))) => (),
            _ => panic!("expected ALU overflow error"),
        }
    }

    #[test]
    fn eval_mul_overflow() {
        let ops = &[
            Stack::Push(Word::MAX).into(),
            Stack::Push(2).into(),
            Alu::Mul.into(),
        ];
        match eval_ops(ops, *test_access()) {
            Err(ConstraintError::Op(_, OpError::Alu(AluError::Overflow))) => (),
            _ => panic!("expected ALU overflow error"),
        }
    }

    #[test]
    fn eval_sub_underflow() {
        let ops = &[
            Stack::Push(Word::MIN).into(),
            Stack::Push(1).into(),
            Alu::Sub.into(),
        ];
        match eval_ops(ops, *test_access()) {
            Err(ConstraintError::Op(_, OpError::Alu(AluError::Underflow))) => (),
            _ => panic!("expected ALU underflow error"),
        }
    }
}
