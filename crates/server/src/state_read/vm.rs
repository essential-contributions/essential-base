use anyhow::bail;
use anyhow::ensure;
use serde::Deserialize;
use serde::Serialize;

use crate::check::pop_one;
use crate::check::pop_two;
use crate::data::Data;
use crate::db::key_range;
use crate::db::Db;
use crate::db::Key;
use crate::db::KeyRange;
use crate::op::Op;
use crate::KeyStore;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum StateReadOp {
    Constraint(Op),
    State(State),
    ControlFlow(ControlFlow),
    Memory(Memory),
    Keys(Keys),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ControlFlow {
    Halt,
    Jump,
    JumpIf,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Memory {
    Alloc,
    Free,
    Load,
    Store,
    Clear,
    IsSome,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Keys {
    Overwrite,
    Push,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum State {
    StateReadWordRange,
    StateReadWordRangeExtern,
}

#[derive(Debug, Clone)]
pub struct ReadOutput {
    pub keys: Vec<KeyRange>,
    pub memory: Vec<Option<u64>>,
}

struct KeysMemory {
    keys: Vec<KeyRange>,
    overwritten: bool,
}

pub fn read(
    db: &Db,
    accounts: &KeyStore,
    data: &Data,
    program: Vec<StateReadOp>,
) -> anyhow::Result<ReadOutput> {
    let mut stack = Vec::new();
    let mut pc = 0;
    let mut running = true;
    let mut memory: Vec<Option<u64>> = Vec::with_capacity(0);
    let mut keys = KeysMemory::new();

    while running {
        let instruction = next_instruction(&program, pc)?;
        match instruction {
            StateReadOp::Constraint(op) => {
                crate::check::eval(&mut stack, accounts, data, op)?;
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
    stack: &mut Vec<u64>,
    db: &Db,
    data: &Data,
    keys: &mut KeysMemory,
    memory: &mut Vec<Option<u64>>,
    pc: &mut usize,
    state: State,
) -> anyhow::Result<()> {
    match state {
        State::StateReadWordRange => {
            let amount = pop_one(stack)?;
            let Some(key_pos) = stack.len().checked_sub(4) else {
                bail!("stack underflow");
            };
            let mut key = [0u64; 4];
            for (s, k) in stack.drain(key_pos..).zip(key.iter_mut()) {
                *k = s;
            }
            keys.track(key, amount);
            let result = db.read_range(&data.this_address, &key, amount as i32);
            ensure!(memory.capacity() >= result.len(), "Memory overflow");
            let start = memory.len();
            memory.extend(result);
            stack.push(start as u64);
            *pc += 1;
        }
        State::StateReadWordRangeExtern => {
            let amount = pop_one(stack)?;
            let Some(key_pos) = stack.len().checked_sub(4) else {
                bail!("stack underflow");
            };
            let mut key = [0u64; 4];
            for (s, k) in stack.drain(key_pos..).zip(key.iter_mut()) {
                *k = s;
            }
            let Some(address_pos) = stack.len().checked_sub(4) else {
                bail!("stack underflow");
            };
            let mut address = [0u64; 4];
            for (s, a) in stack.drain(address_pos..).zip(address.iter_mut()) {
                *a = s;
            }
            let result = db.read_range(&address, &key, amount as i32);
            ensure!(memory.capacity() >= result.len(), "Memory overflow");
            let start = memory.len();
            memory.extend(result);
            stack.push(start as u64);
            *pc += 1;
        }
    }
    Ok(())
}

fn eval_control_flow(
    stack: &mut Vec<u64>,
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
            }
        }
    }
    Ok(())
}

fn eval_memory(
    stack: &mut Vec<u64>,
    pc: &mut usize,
    memory: &mut Vec<Option<u64>>,
    op: Memory,
) -> anyhow::Result<()> {
    match op {
        Memory::Alloc => {
            let size = pop_one(stack)?;
            memory.reserve(size as usize);
            *pc += 1;
        }
        Memory::Free => {
            let size = pop_one(stack)?;
            let new_size = memory.capacity().saturating_sub(size as usize);
            memory.shrink_to(new_size);
            *pc += 1;
        }
        Memory::Load => {
            let index = pop_one(stack)?;
            let value = memory
                .get(index as usize)
                .copied()
                .flatten()
                .ok_or_else(|| anyhow::anyhow!("invalid memory access"))?;
            stack.push(value);
            *pc += 1;
        }
        Memory::Store => {
            let (index, value) = pop_two(stack)?;
            match memory.get_mut(index as usize) {
                Some(m) => *m = Some(value),
                None => bail!("index out of bounds"),
            }
            *pc += 1;
        }
        Memory::Clear => {
            let index = pop_one(stack)?;
            match memory.get_mut(index as usize) {
                Some(m) => *m = None,
                None => bail!("index out of bounds"),
            }
            *pc += 1;
        }
        Memory::IsSome => {
            let index = pop_one(stack)?;
            let value = memory.get(index as usize).copied().flatten().is_some() as u64;
            stack.push(value);
            *pc += 1;
        }
    }
    Ok(())
}

fn eval_keys(
    stack: &mut Vec<u64>,
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
            let mut key = [0u64; 4];
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

    fn track(&mut self, key: Key, amount: u64) {
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

    fn push(&mut self, key: Key, amount: u64) {
        if let Some(kr) = key_range(key, amount) {
            self.keys.push(kr);
        }
    }
}
