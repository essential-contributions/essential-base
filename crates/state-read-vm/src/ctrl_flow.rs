//! ControlFlow operation implementations.

use crate::{
    error::{ControlFlowError, OpSyncError, OpSyncResult, StackError},
    types::convert::bool_from_word,
    Vm,
};

/// `ControlFlow::Jump` operation.
pub fn jump(vm: &mut Vm) -> Result<usize, StackError> {
    let new_pc = vm.stack.pop()?;
    usize::try_from(new_pc).map_err(|_| StackError::IndexOutOfBounds)
}

/// `ControlFlow::JumpIf` operation.
pub fn jump_if(vm: &mut Vm) -> OpSyncResult<usize> {
    let [new_pc, cond] = vm.stack.pop2()?;
    let cond = bool_from_word(cond).ok_or(ControlFlowError::InvalidJumpIfCondition(cond))?;
    let new_pc = match cond {
        true => usize::try_from(new_pc).map_err(|_| StackError::IndexOutOfBounds)?,
        false => vm.pc.checked_add(1).ok_or(OpSyncError::PcOverflow)?,
    };
    Ok(new_pc)
}
