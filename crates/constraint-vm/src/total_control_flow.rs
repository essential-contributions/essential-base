use essential_types::convert::bool_from_word;

use crate::{
    error::{StackError, TotalControlFlowError},
    OpResult, Stack,
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Update the program to a new position or halt it.
pub enum UpdateProgram {
    /// New program counter position.
    Pc(usize),
    /// Halt the program.
    Halt,
}

pub fn jump_forward_if(stack: &mut Stack, pc: &usize) -> OpResult<Option<UpdateProgram>> {
    let [dist, cond] = stack.pop2()?;
    let cond = bool_from_word(cond).ok_or(TotalControlFlowError::InvalidJumpForwardIfCondition)?;
    if cond {
        let dist = usize::try_from(dist).map_err(|_| StackError::IndexOutOfBounds)?;
        let pc = pc.checked_add(dist).ok_or(StackError::IndexOutOfBounds)?;
        Ok(Some(UpdateProgram::Pc(pc)))
    } else {
        Ok(None)
    }
}

pub fn halt_if(stack: &mut Stack) -> OpResult<Option<UpdateProgram>> {
    let cond = stack.pop()?;
    let cond = bool_from_word(cond).ok_or(TotalControlFlowError::InvalidHaltIfCondition)?;
    if cond {
        Ok(Some(UpdateProgram::Halt))
    } else {
        Ok(None)
    }
}
