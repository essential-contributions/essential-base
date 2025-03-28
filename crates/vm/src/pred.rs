use crate::{
    error::{OpError, OpResult, StackError},
    sets::decode_set,
    Stack,
};
use essential_types::Word;
use std::collections::HashSet;

#[cfg(test)]
mod tests;

/// `Pred::EqRange` implementation.
pub(crate) fn eq_range(stack: &mut Stack) -> OpResult<()> {
    // Pop the length off the stack.
    let len = stack.pop()?;

    // If the length is 0, the ranges are equal.
    if len == 0 {
        stack.push(1)?;
        return Ok(());
    }

    let double = len.checked_mul(2).ok_or(StackError::IndexOutOfBounds)?;
    let len: usize = len.try_into().map_err(|_| StackError::IndexOutOfBounds)?;

    stack.push(double)?;

    let eq = stack.pop_len_words::<_, _, OpError>(|words| {
        let (a, b) = words.split_at(len);
        Ok(a == b)
    })?;

    // Push the result back onto the stack.
    stack.push(eq.into())?;

    Ok(())
}

/// `Pred::EqSet` implementation.
pub(crate) fn eq_set(stack: &mut Stack) -> OpResult<()> {
    let eq = stack.pop_len_words2::<_, _, OpError>(|lhs, rhs| {
        let lhs = decode_set(lhs).collect::<Result<HashSet<&[Word]>, _>>()?;
        let rhs = decode_set(rhs).collect::<Result<HashSet<&[Word]>, _>>()?;
        Ok(lhs == rhs)
    })?;
    stack.push(eq.into())?;
    Ok(())
}

#[cfg(test)]
mod pred_tests {
    use crate::{
        asm::{Pred, Stack},
        sync::{eval_ops, test_util::*},
        utils::EmptyState,
        GasLimit, Op,
    };

    #[test]
    fn pred_eq_false() {
        let ops = &[
            Stack::Push(6).into(),
            Stack::Push(7).into(),
            Pred::Eq.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(!eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_eq_true() {
        let ops = &[
            Stack::Push(42).into(),
            Stack::Push(42).into(),
            Pred::Eq.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_gt_false() {
        let ops = &[
            Stack::Push(7).into(),
            Stack::Push(7).into(),
            Pred::Gt.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(!eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_gt_true() {
        let ops = &[
            Stack::Push(7).into(),
            Stack::Push(6).into(),
            Pred::Gt.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_lt_false() {
        let ops = &[
            Stack::Push(7).into(),
            Stack::Push(7).into(),
            Pred::Lt.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(!eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_lt_true() {
        let ops = &[
            Stack::Push(6).into(),
            Stack::Push(7).into(),
            Pred::Lt.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_gte_false() {
        let ops = &[
            Stack::Push(6).into(),
            Stack::Push(7).into(),
            Pred::Gte.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(!eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_gte_true() {
        let ops = &[
            Stack::Push(7).into(),
            Stack::Push(7).into(),
            Pred::Gte.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
        let ops = &[
            Stack::Push(8).into(),
            Stack::Push(7).into(),
            Pred::Gte.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_lte_false() {
        let ops = &[
            Stack::Push(7).into(),
            Stack::Push(6).into(),
            Pred::Lte.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(!eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_lte_true() {
        let ops = &[
            Stack::Push(7).into(),
            Stack::Push(7).into(),
            Pred::Lte.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
        let ops = &[
            Stack::Push(7).into(),
            Stack::Push(8).into(),
            Pred::Lte.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_and_true() {
        let ops = &[
            Stack::Push(42).into(),
            Stack::Push(42).into(),
            Pred::And.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_and_false() {
        let ops = &[
            Stack::Push(42).into(),
            Stack::Push(0).into(),
            Pred::And.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(!eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
        let ops = &[
            Stack::Push(0).into(),
            Stack::Push(0).into(),
            Pred::And.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(!eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_or_true() {
        let ops = &[
            Stack::Push(42).into(),
            Stack::Push(42).into(),
            Pred::Or.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
        let ops = &[
            Stack::Push(0).into(),
            Stack::Push(42).into(),
            Pred::Or.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
        let ops = &[
            Stack::Push(42).into(),
            Stack::Push(0).into(),
            Pred::Or.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_or_false() {
        let ops = &[
            Stack::Push(0).into(),
            Stack::Push(0).into(),
            Pred::Or.into(),
        ];
        let op_gas_cost = &|_: &Op| 1;
        assert!(!eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_not_true() {
        let ops = &[Stack::Push(0).into(), Pred::Not.into()];
        let op_gas_cost = &|_: &Op| 1;
        assert!(eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }

    #[test]
    fn pred_not_false() {
        let ops = &[Stack::Push(42).into(), Pred::Not.into()];
        let op_gas_cost = &|_: &Op| 1;
        assert!(!eval_ops(
            ops,
            test_access().clone(),
            &EmptyState,
            op_gas_cost,
            GasLimit::UNLIMITED
        )
        .unwrap());
    }
}
