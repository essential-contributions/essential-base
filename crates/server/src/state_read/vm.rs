use anyhow::bail;
use anyhow::ensure;
use essential_types::Word;

use crate::check::pop_one;
use crate::check::pop_two;
use crate::data::Data;
use crate::db::key_range;
use crate::db::Db;
use crate::db::Key;
use crate::db::KeyRange;

use state_asm::*;

#[derive(Debug, Clone)]
pub struct ReadOutput {
    pub keys: Vec<KeyRange>,
    pub memory: Vec<Option<Word>>,
}

struct KeysMemory {
    keys: Vec<KeyRange>,
    overwritten: bool,
}

pub fn read(db: &Db, data: &Data, program: Vec<StateReadOp>) -> anyhow::Result<ReadOutput> {
    let mut stack = Vec::new();
    let mut pc = 0;
    let mut running = true;
    let mut memory: Vec<Option<Word>> = Vec::with_capacity(0);
    let mut keys = KeysMemory::new();

    while running {
        let instruction = next_instruction(&program, pc)?;
        match instruction {
            StateReadOp::Constraint(op) => {
                crate::check::eval(&mut stack, data, op)?;
                pc += 1;
            }
            StateReadOp::State(state) => {
                eval_state(&mut stack, db, data, &mut keys, &mut memory, &mut pc, state)?;
            }
            StateReadOp::ControlFlow(cf) => {
                eval_control_flow(&mut stack, &mut pc, &mut running, cf)?
            }
            StateReadOp::Memory(mem) => {
                eval_memory(&mut stack, &mut pc, &mut memory, mem)?;
            }
            StateReadOp::Keys(k) => {
                eval_keys(&mut stack, &mut pc, &mut keys, k)?;
            }
        }
        if !matches!(instruction, StateReadOp::Constraint(_)) {
            println!("Op: {:?}, Stack: {:?}", instruction, stack);
        }
    }
    Ok(ReadOutput {
        keys: keys.keys,
        memory,
    })
}

fn eval_state(
    stack: &mut Vec<Word>,
    db: &Db,
    data: &Data,
    keys: &mut KeysMemory,
    memory: &mut Vec<Option<Word>>,
    pc: &mut usize,
    state: State,
) -> anyhow::Result<()> {
    match state {
        State::StateReadWordRange => {
            let amount = pop_one(stack)?;
            let Some(key_pos) = stack.len().checked_sub(4) else {
                bail!("stack underflow");
            };
            let mut key = [0i64; 4];
            for (s, k) in stack.drain(key_pos..).zip(key.iter_mut()) {
                *k = s;
            }
            keys.track(key, amount);
            let amount: i32 = amount.try_into()?;
            let result = db.read_range(
                &data.source_address.set_address().clone().into(),
                &key,
                amount,
            );
            ensure!(memory.capacity() >= result.len(), "Memory overflow");
            let start = memory.len();
            memory.extend(result);
            stack.push(start as Word);
            *pc += 1;
        }
        State::StateReadWordRangeExtern => {
            let amount = pop_one(stack)?;
            let Some(key_pos) = stack.len().checked_sub(4) else {
                bail!("stack underflow");
            };
            let mut key = [0i64; 4];
            for (s, k) in stack.drain(key_pos..).zip(key.iter_mut()) {
                *k = s;
            }
            let Some(address_pos) = stack.len().checked_sub(4) else {
                bail!("stack underflow");
            };
            let mut address = [0i64; 4];
            for (s, a) in stack.drain(address_pos..).zip(address.iter_mut()) {
                *a = s;
            }
            let result = db.read_range(&address, &key, amount as i32);
            ensure!(memory.capacity() >= result.len(), "Memory overflow");
            let start = memory.len();
            memory.extend(result);
            stack.push(start as Word);
            *pc += 1;
        }
    }
    Ok(())
}

fn eval_control_flow(
    stack: &mut Vec<Word>,
    pc: &mut usize,
    running: &mut bool,
    cf: ControlFlow,
) -> anyhow::Result<()> {
    match cf {
        ControlFlow::Halt => {
            *running = false;
        }
        ControlFlow::Jump => {
            let new_pc = pop_one(stack)?;
            *pc = new_pc as usize;
        }
        ControlFlow::JumpIf => {
            let (new_pc, cond) = pop_two(stack)?;
            if cond != 0 {
                *pc = new_pc as usize;
            } else {
                *pc += 1;
            }
        }
    }
    Ok(())
}

fn eval_memory(
    stack: &mut Vec<Word>,
    pc: &mut usize,
    memory: &mut Vec<Option<Word>>,
    op: Memory,
) -> anyhow::Result<()> {
    match op {
        Memory::Alloc => {
            let size = pop_one(stack)?;
            let size: usize = size.try_into()?;
            memory.reserve_exact(size);
            *pc += 1;
        }
        Memory::Free => {
            let size = pop_one(stack)?;
            let size: usize = size.try_into()?;
            let new_size = memory.capacity().saturating_sub(size);
            memory.shrink_to(new_size);
            *pc += 1;
        }
        Memory::Truncate => {
            let size = pop_one(stack)?;
            let size: usize = size.try_into()?;
            memory.truncate(size);
            *pc += 1;
        }
        Memory::Load => {
            let index = pop_one(stack)?;
            let index: usize = index.try_into()?;
            let value = memory.get(index);
            match value {
                Some(v) => stack.push(v.unwrap_or_default()),
                None => bail!("index out of bounds"),
            }
            *pc += 1;
        }
        Memory::Store => {
            let (index, value) = pop_two(stack)?;
            let index: usize = index.try_into()?;
            match memory.get_mut(index) {
                Some(m) => *m = Some(value),
                None => bail!("index out of bounds"),
            }
            *pc += 1;
        }
        Memory::Push => {
            let value = pop_one(stack)?;
            ensure!(memory.capacity() > memory.len(), "Memory overflow");
            memory.push(Some(value));
            *pc += 1;
        }
        Memory::PushNone => {
            ensure!(memory.capacity() > memory.len(), "Memory overflow");
            memory.push(None);
            *pc += 1;
        }
        Memory::Clear => {
            let index = pop_one(stack)?;
            let index: usize = index.try_into()?;
            match memory.get_mut(index) {
                Some(m) => *m = None,
                None => bail!("index out of bounds"),
            }
            *pc += 1;
        }
        Memory::ClearRange => {
            let (index, amount) = pop_two(stack)?;
            let index: usize = index.try_into()?;
            let amount: usize = amount.try_into()?;
            let Some(end) = index.checked_add(amount) else {
                bail!("index out of bounds");
            };
            match memory.get_mut(index..end) {
                Some(mem) => {
                    for m in mem {
                        *m = None;
                    }
                }
                None => bail!("index out of bounds"),
            }
            *pc += 1;
        }
        Memory::IsSome => {
            let index = pop_one(stack)?;
            let index: usize = index.try_into()?;
            let value = memory.get(index).map(|v| v.is_some() as Word);
            match value {
                Some(v) => stack.push(v),
                None => bail!("index out of bounds"),
            }
            *pc += 1;
        }
        Memory::Capacity => {
            stack.push(memory.capacity() as Word);
            *pc += 1;
        }
        Memory::Length => {
            stack.push(memory.len() as Word);
            *pc += 1;
        }
    }
    Ok(())
}

fn eval_keys(
    stack: &mut Vec<Word>,
    pc: &mut usize,
    keys: &mut KeysMemory,
    k: Keys,
) -> anyhow::Result<()> {
    match k {
        Keys::Overwrite => {
            keys.overwrite();
            *pc += 1;
        }
        Keys::Push => {
            let amount = pop_one(stack)?;
            let Some(key_pos) = stack.len().checked_sub(4) else {
                bail!("stack underflow");
            };
            let mut key = [0i64; 4];
            for (s, k) in stack.drain(key_pos..).zip(key.iter_mut()) {
                *k = s;
            }
            keys.push(key, amount);
            *pc += 1;
        }
    }
    Ok(())
}

fn next_instruction(program: &[StateReadOp], pc: usize) -> anyhow::Result<StateReadOp> {
    program
        .get(pc)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("pc out of bounds"))
}

impl KeysMemory {
    fn new() -> Self {
        Self {
            keys: Vec::new(),
            overwritten: false,
        }
    }

    fn track(&mut self, key: Key, amount: Word) {
        if self.overwritten {
            return;
        }
        if let Some(kr) = key_range(key, amount) {
            self.keys.push(kr);
        }
    }

    fn overwrite(&mut self) {
        self.overwritten = true;
        self.keys.clear();
    }

    fn push(&mut self, key: Key, amount: Word) {
        if let Some(kr) = key_range(key, amount) {
            self.keys.push(kr);
        }
    }
}
