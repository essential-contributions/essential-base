use anyhow::bail;
use anyhow::ensure;

use crate::data::Data;
use crate::data::InputMessage;
use crate::data::OutputMessage;
use crate::data::Slots;
use crate::db::Address;
use crate::db::Db;
use crate::db::Key;
use crate::op::Access;
use crate::op::Alu;
use crate::op::Op;
use crate::op::Pred;
use crate::state_read;
use crate::state_read::load_module;
use crate::state_read::vm;
use crate::state_read::vm::StateReadOp;
use crate::state_read::StateSlot;
use crate::state_read::VmCall;
use crate::state_read::WasmCall;
use crate::Intent;

pub struct SolvedIntent {
    pub intent: Intent,
    pub solution: Solution,
}

pub enum Directive {
    Satisfy,
    Maximize(Vec<u8>),
    Minimize(Vec<u8>),
}

#[derive(Debug, Default, Clone)]
pub struct Solution {
    pub decision_variables: Vec<u64>,
    pub input_message: InputMessage,
    pub output_messages: Vec<OutputMessage>,
    pub state_mutations: Vec<(Key, Option<u64>)>,
}

impl SolvedIntent {
    pub fn address(&self) -> Address {
        self.intent.address()
    }
}

pub fn check(db: &mut Db, intent: SolvedIntent) -> anyhow::Result<u64> {
    check_slots(&intent.intent.slots, &intent.solution)?;
    let len = intent.intent.slots.state.len();
    let mut state = vec![None; len];
    let mut state_delta = vec![None; len];

    let mut data = Data {
        this_address: intent.address(),
        decision_variables: intent.solution.decision_variables.clone(),
        state: state.clone(),
        state_delta: state_delta.clone(),
        input_message: intent.solution.input_message.clone(),
        output_messages: intent.solution.output_messages.clone(),
    };

    read_state(&intent.intent, db.clone(), &mut data, &mut state, false)?;

    for (key, value) in intent.solution.state_mutations {
        db.stage(data.this_address, key, value);
    }

    read_state(
        &intent.intent,
        db.clone(),
        &mut data,
        &mut state_delta,
        true,
    )?;

    check_constraints(&data, &intent.intent.constraints)?;

    match intent.intent.directive {
        Directive::Satisfy => Ok(1),
        Directive::Maximize(code) | Directive::Minimize(code) => {
            let ops = serde_json::from_slice(&code)?;
            pop_one(&mut run(&data, ops)?)
        }
    }
}

fn read_state(
    intent: &Intent,
    db: Db,
    data: &mut Data,
    state: &mut [Option<u64>],
    delta: bool,
) -> anyhow::Result<()> {
    match (&intent.state_read, &intent.slots.state) {
        (state_read::StateRead::Wasm(read), state_read::StateRead::Wasm(state_slots)) => {
            read_state_wasm(read, db, intent.address(), state_slots, data, state, delta)?
        }
        (state_read::StateRead::Vm(read), state_read::StateRead::Vm(state_slots)) => {
            read_state_vm(read, db, state_slots, data, state, delta)?
        }
        _ => bail!("State read mismatch"),
    }
    Ok(())
}

fn read_state_wasm(
    read: &[u8],
    db: Db,
    this_address: Address,
    state_slots: &[StateSlot<WasmCall>],
    data: &mut Data,
    state: &mut [Option<u64>],
    delta: bool,
) -> anyhow::Result<()> {
    if !read.is_empty() {
        let (mut store, module) = load_module(this_address, read, db)?;
        for slot in state_slots {
            let mut params = Vec::with_capacity(slot.call.params.len());
            for param in &slot.call.params {
                let ops = serde_json::from_slice(param)?;
                let stack = run(data, ops)?;
                params.push(stack);
            }
            let result =
                state_read::read_state(&mut store, &module, &slot.call.fn_name, params.clone())?;
            if result.len() != slot.amount as usize {
                bail!("State read failed");
            }
            for (s, r) in state.iter_mut().skip(slot.index as usize).zip(result) {
                *s = r;
            }
            if delta {
                data.state_delta = state.to_vec();
            } else {
                data.state = state.to_vec();
            }
        }
    }
    Ok(())
}

fn read_state_vm(
    read: &[u8],
    db: Db,
    state_slots: &[StateSlot<VmCall>],
    data: &mut Data,
    state: &mut [Option<u64>],
    delta: bool,
) -> anyhow::Result<()> {
    if !read.is_empty() {
        let programs: Vec<Vec<StateReadOp>> = serde_json::from_slice(read)?;
        for slot in state_slots {
            let Some(program) = programs.get(slot.call.index as usize) else {
                bail!("State read program out of bounds");
            };
            let result = vm::read(&db, data, program.clone())?;
            if result.len() != slot.amount as usize {
                bail!("State read failed");
            }
            for (s, r) in state.iter_mut().skip(slot.index as usize).zip(result) {
                *s = r;
            }
            if delta {
                data.state_delta = state.to_vec();
            } else {
                data.state = state.to_vec();
            }
        }
    }
    Ok(())
}

fn check_slots(slots: &Slots, solution: &Solution) -> anyhow::Result<()> {
    ensure!(slots.decision_variables == solution.decision_variables.len() as u64);
    ensure!(slots.input_message_args.len() == solution.input_message.args.len());
    ensure!(slots.output_messages_args.len() == solution.output_messages.len());
    for (expected, args) in slots
        .input_message_args
        .iter()
        .zip(solution.input_message.args.iter())
    {
        ensure!(*expected == args.len() as u64);
    }
    for (expected, args) in slots
        .output_messages_args
        .iter()
        .zip(solution.output_messages.iter())
    {
        ensure!(expected.len() == args.args.len());
        for (len, got) in expected.iter().zip(args.args.iter()) {
            ensure!(*len == got.len() as u64);
        }
    }
    Ok(())
}

fn check_constraints(data: &Data, constraints: &Vec<Vec<u8>>) -> anyhow::Result<()> {
    for constraint in constraints {
        let ops = serde_json::from_slice(constraint)?;
        check_constraint(data, ops)?;
    }

    Ok(())
}

fn check_constraint(data: &Data, ops: Vec<Op>) -> anyhow::Result<()> {
    let mut output = run(data, ops)?;
    let output = pop_one(&mut output)?;

    if output != 1 {
        anyhow::bail!("Constraint failed");
    }

    Ok(())
}

fn run(data: &Data, ops: Vec<Op>) -> anyhow::Result<Vec<u64>> {
    let mut stack = Vec::new();
    println!("Result: {:?}", stack);
    for op in ops {
        eval(&mut stack, data, op)?;
    }
    Ok(stack)
}

pub fn eval(stack: &mut Vec<u64>, data: &Data, op: Op) -> anyhow::Result<()> {
    match op {
        Op::Push(word) => {
            stack.push(word);
        }
        Op::Pop => {
            stack.pop();
        }
        Op::Dup => {
            let word = pop_one(stack)?;
            stack.push(word);
            stack.push(word);
        }
        Op::Swap => {
            let (word1, word2) = pop_two(stack)?;
            stack.push(word1);
            stack.push(word2);
        }
        Op::Pred(pred) => check_predicate(stack, pred)?,
        Op::Alu(alu) => check_alu(stack, alu)?,
        Op::Access(access) => check_access(data, stack, access)?,
    }
    println!("Op: {:?}, Stack: {:?}", op, stack);
    Ok(())
}

fn check_predicate(stack: &mut Vec<u64>, pred: Pred) -> anyhow::Result<()> {
    let word1 = pop_one(stack)?;
    let result = match pred {
        Pred::Eq => pop_one(stack)? == word1,
        Pred::Gt => pop_one(stack)? > word1,
        Pred::Lt => pop_one(stack)? < word1,
        Pred::Gte => pop_one(stack)? >= word1,
        Pred::Lte => pop_one(stack)? <= word1,
        Pred::And => {
            let word2 = pop_one(stack)?;
            word1 != 0 && word2 != 0
        }
        Pred::Or => {
            let word2 = pop_one(stack)?;
            word1 != 0 || word2 != 0
        }
        Pred::Not => word1 == 0,
    };
    stack.push(result as u64);
    Ok(())
}

fn check_alu(stack: &mut Vec<u64>, alu: Alu) -> anyhow::Result<()> {
    let (word1, word2) = pop_two(stack)?;
    let result = match alu {
        Alu::Add => word1 + word2,
        Alu::Sub => word1 - word2,
        Alu::Mul => word1 * word2,
        Alu::Div => word1 / word2,
        Alu::Mod => word1 % word2,
    };
    stack.push(result);
    Ok(())
}

fn check_access(data: &Data, stack: &mut Vec<u64>, access: Access) -> anyhow::Result<()> {
    match access {
        Access::DecisionVar => {
            let address = pop_one(stack)?;
            let Some(var) = data.decision_variables.get(address as usize) else {
                bail!("Decision variable out of bounds");
            };
            stack.push(*var);
        }
        Access::DecisionVarRange => {
            let (index, range) = pop_two(stack)?;
            let Some(slice) = data
                .decision_variables
                .get(index as usize..(index + range) as usize)
            else {
                bail!("Decision variable range out of bounds");
            };
            stack.extend(slice);
        }
        Access::State => {
            let (address, delta) = pop_two(stack)?;
            let state = match delta {
                0 => data.state.get(address as usize).copied().flatten(),
                1 => data.state_delta.get(address as usize).copied().flatten(),
                _ => anyhow::bail!("Invalid state access"),
            };
            if let Some(state) = state {
                stack.push(state);
            }
        }
        Access::StateRange => {
            let (address, range) = pop_two(stack)?;
            match pop_one(stack)? {
                0 => {
                    let iter = data.state[address as usize..(address + range) as usize]
                        .iter()
                        .flatten();
                    stack.extend(iter);
                }
                1 => {
                    let iter = data.state_delta[address as usize..(address + range) as usize]
                        .iter()
                        .flatten();
                    stack.extend(iter);
                }
                _ => anyhow::bail!("Invalid state access"),
            }
        }
        Access::StateIsSome => {
            let (address, delta) = pop_two(stack)?;
            let state = match delta {
                0 => data.state.get(address as usize).copied().flatten(),
                1 => data.state_delta.get(address as usize).copied().flatten(),
                _ => anyhow::bail!("Invalid state access"),
            };
            stack.push(state.is_some() as u64);
        }
        Access::StateIsSomeRange => {
            let (address, range) = pop_two(stack)?;
            match pop_one(stack)? {
                0 => {
                    let iter = data.state[address as usize..(address + range) as usize]
                        .iter()
                        .map(|i| i.is_some() as u64);
                    stack.extend(iter);
                }
                1 => {
                    let iter = data.state_delta[address as usize..(address + range) as usize]
                        .iter()
                        .map(|i| i.is_some() as u64);
                    stack.extend(iter);
                }
                _ => anyhow::bail!("Invalid state access"),
            }
        }
        Access::InputMsgSenderWord => {
            let index = pop_one(stack)?;
            stack.push(data.input_message.sender[index as usize]);
        }
        Access::InputMsgSender => {
            stack.extend(data.input_message.sender);
        }
        Access::InputMsgArgWord => {
            let (arg_index, word_index) = pop_two(stack)?;
            stack.push(data.input_message.args[arg_index as usize][word_index as usize]);
        }
        Access::InputMsgArgRange => {
            let (start, end) = pop_two(stack)?;
            let index = pop_one(stack)?;
            stack.extend(&data.input_message.args[index as usize][start as usize..end as usize]);
        }
        Access::InputMsgArg => {
            let index = pop_one(stack)?;
            stack.extend(&data.input_message.args[index as usize]);
        }
        Access::OutputMsgRecipientWord => {
            let (msg_index, word_index) = pop_two(stack)?;
            stack.push(data.output_messages[msg_index as usize].recipient[word_index as usize]);
        }
        Access::OutputMsgRecipient => {
            let msg_index = pop_one(stack)?;
            stack.extend(data.output_messages[msg_index as usize].recipient);
        }
        Access::OutputMsgArgWord => {
            let (arg_index, word_index) = pop_two(stack)?;
            let msg_index = pop_one(stack)?;
            let word = data.output_messages[msg_index as usize].args[arg_index as usize]
                [word_index as usize];
            stack.push(word);
        }
        Access::OutputMsgArgRange => {
            let (start, end) = pop_two(stack)?;
            let (msg_index, arg_index) = pop_two(stack)?;
            let iter = &data.output_messages[msg_index as usize].args[arg_index as usize]
                [start as usize..end as usize];
            stack.extend(iter);
        }
        Access::OutputMsgArg => {
            let (msg_index, arg_index) = pop_two(stack)?;
            let iter = &data.output_messages[msg_index as usize].args[arg_index as usize];
            stack.extend(iter);
        }
    }
    Ok(())
}

pub fn pop_one(stack: &mut Vec<u64>) -> anyhow::Result<u64> {
    stack
        .pop()
        .ok_or_else(|| anyhow::anyhow!("Stack underflow"))
}

pub fn pop_two(stack: &mut Vec<u64>) -> anyhow::Result<(u64, u64)> {
    let word1 = pop_one(stack)?;
    let word2 = pop_one(stack)?;
    Ok((word2, word1))
}
