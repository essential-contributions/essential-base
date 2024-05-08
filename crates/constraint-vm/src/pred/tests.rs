use essential_types::solution::{DecisionVariable, SolutionData};

use crate::{
    test_util::{test_empty_keys, test_solution_access, TEST_INTENT_ADDR},
    SolutionAccess, StateSlots,
};

use super::*;

#[test]
fn test_eq_range_true() {
    let mut stack = Stack::default();
    stack.extend([1, 2, 3]).unwrap();
    stack.extend([1, 2, 3]).unwrap();
    stack.push(5).unwrap();
    stack.push(2).unwrap();
    stack.push(3).unwrap();
    eq_range(&mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 1);
}

#[test]
fn test_eq_range_false() {
    let mut stack = Stack::default();
    stack.extend([1, 4, 3]).unwrap();
    stack.extend([1, 2, 3]).unwrap();
    stack.push(5).unwrap();
    stack.push(2).unwrap();
    stack.push(3).unwrap();
    eq_range(&mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);
}

#[test]
fn test_eq_empty_range() {
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    eq_range(&mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 1);
}

#[test]
fn test_eq_range_dec_var_true() {
    let data = vec![SolutionData {
        intent_to_solve: TEST_INTENT_ADDR,
        decision_variables: vec![
            DecisionVariable::Inline(3),
            DecisionVariable::Inline(1),
            DecisionVariable::Inline(2),
            DecisionVariable::Inline(3),
        ],
    }];
    let access = Access {
        solution: SolutionAccess {
            data: &data,
            index: 0,
            mutable_keys: test_empty_keys(),
        },
        state_slots: StateSlots::EMPTY,
    };
    let mut stack = Stack::default();
    stack.extend([1, 2, 3]).unwrap();
    stack.push(2).unwrap();
    stack.push(1).unwrap();
    stack.push(3).unwrap();
    eq_range_dec_var(access, &mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 1);
}

#[test]
fn test_eq_range_dec_var_false() {
    let data = vec![SolutionData {
        intent_to_solve: TEST_INTENT_ADDR,
        decision_variables: vec![
            DecisionVariable::Inline(3),
            DecisionVariable::Inline(1),
            DecisionVariable::Inline(4),
            DecisionVariable::Inline(3),
        ],
    }];
    let access = Access {
        solution: SolutionAccess {
            data: &data,
            index: 0,
            mutable_keys: test_empty_keys(),
        },
        state_slots: StateSlots::EMPTY,
    };
    let mut stack = Stack::default();
    stack.extend([1, 2, 3]).unwrap();
    stack.push(2).unwrap();
    stack.push(1).unwrap();
    stack.push(3).unwrap();
    eq_range_dec_var(access, &mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);
}

#[test]
fn test_eq_range_state_true() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[Some(1), Some(2), Some(3)],
            post: &[Some(0), Some(0)],
        },
    };
    let mut stack = Stack::default();
    stack.extend([1, 2, 3]).unwrap();
    stack.push(2).unwrap();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(3).unwrap();
    eq_range_state(access, &mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 1);
}

#[test]
fn test_eq_range_state_false() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[Some(1), Some(4), Some(3)],
            post: &[Some(0), Some(0)],
        },
    };
    let mut stack = Stack::default();
    stack.extend([1, 2, 3]).unwrap();
    stack.push(2).unwrap();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(3).unwrap();
    eq_range_state(access, &mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);
}

#[test]
fn test_eq_range_state_true_post() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[Some(0), Some(0)],
            post: &[Some(1), Some(2), Some(3)],
        },
    };
    let mut stack = Stack::default();
    stack.extend([1, 2, 3]).unwrap();
    stack.push(2).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    stack.push(3).unwrap();
    eq_range_state(access, &mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 1);
}

#[test]
fn test_eq_range_state_out_of_range() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[Some(0), Some(0)],
            post: &[Some(1), Some(2), Some(3)],
        },
    };
    let mut stack = Stack::default();
    stack.extend([1, 2, 3]).unwrap();
    stack.push(2).unwrap();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(3).unwrap();
    eq_range_state(access, &mut stack).unwrap_err();
}
