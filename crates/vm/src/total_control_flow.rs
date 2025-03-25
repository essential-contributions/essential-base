use essential_types::convert::bool_from_word;

use crate::{
    error::{OpError, OpResult, StackError, TotalControlFlowError},
    Stack,
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Update the program to a new position or halt it.
pub enum ProgramControlFlow {
    /// New program counter position.
    Pc(usize),
    /// Halt the program.
    Halt,
}

pub fn jump_if(stack: &mut Stack, pc: usize) -> OpResult<Option<ProgramControlFlow>> {
    let [dist, cond] = stack.pop2()?;
    let cond = bool_from_word(cond).ok_or(TotalControlFlowError::InvalidJumpForwardIfCondition)?;
    if cond {
        let neg = dist < 0;
        let dist = usize::try_from(dist.abs()).map_err(|_| StackError::IndexOutOfBounds)?;
        if dist == 0 {
            return Err(TotalControlFlowError::JumpedToSelf.into());
        }
        if neg {
            let pc = pc.checked_sub(dist).ok_or(OpError::PcOverflow)?;
            Ok(Some(ProgramControlFlow::Pc(pc)))
        } else {
            let pc = pc.checked_add(dist).ok_or(OpError::PcOverflow)?;
            Ok(Some(ProgramControlFlow::Pc(pc)))
        }
    } else {
        Ok(None)
    }
}

pub fn halt_if(stack: &mut Stack) -> OpResult<Option<ProgramControlFlow>> {
    let cond = stack.pop()?;
    let cond = bool_from_word(cond).ok_or(TotalControlFlowError::InvalidHaltIfCondition)?;
    if cond {
        Ok(Some(ProgramControlFlow::Halt))
    } else {
        Ok(None)
    }
}

/// Implementation of the `PanicIf` operation.
pub fn panic_if(stack: &mut Stack) -> OpResult<()> {
    let cond = stack.pop()?;
    let cond = bool_from_word(cond).ok_or(TotalControlFlowError::InvalidPanicIfCondition)?;
    if cond {
        let stack = stack.iter().copied().collect();
        Err(TotalControlFlowError::Panic(stack).into())
    } else {
        Ok(())
    }
}
