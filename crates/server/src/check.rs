use anyhow::bail;
use anyhow::ensure;
use essential_types::solution::KeyMutation;
use essential_types::solution::Mutation;
use essential_types::solution::RangeMutation;
use essential_types::solution::SolutionData;
use essential_types::solution::StateMutation;

use crate::data::Data;
use crate::data::Slots;
use crate::db::add_to_key;
use crate::db::Address;
use crate::db::Db;
use crate::db::KeyRange;
use crate::db::KeyRangeIter;
use crate::intent::Intent;
use crate::intent::ToIntentAddress;
use crate::state_read::vm;
use crate::state_read::vm::ReadOutput;
use crate::state_read::StateSlot;
use state_asm::constraint_asm::Access;
use state_asm::constraint_asm::Alu;
use state_asm::constraint_asm::Crypto;
use state_asm::constraint_asm::Op;
use state_asm::constraint_asm::Pred;
use state_asm::StateReadOp;

pub use essential_types::intent::Directive;

#[cfg(test)]
mod tests;

pub struct SolvedIntent {
    pub intent: Intent,
    pub solution: SolutionData,
    pub state_mutations: Vec<StateMutation>,
}

impl SolvedIntent {
    pub fn address(&self) -> Address {
        self.intent.address()
    }
}

pub fn check(db: &mut Db, intent: SolvedIntent) -> anyhow::Result<u64> {
    check_slots(&intent.intent.slots, &intent.solution)?;
    let len =
        essential_types::slots::state_len(&intent.intent.slots.state).unwrap_or_default() as usize;
    let mut state = vec![None; len];
    let mut state_delta = vec![None; len];

    let mut data = Data {
        this_address: intent.address(),
        decision_variables: intent.solution.decision_variables.clone(),
        state: state.clone(),
        state_delta: state_delta.clone(),
        input_message: intent.solution.input_message.clone().map(Into::into),
        output_messages: intent
            .solution
            .output_messages
            .clone()
            .into_iter()
            .map(Into::into)
            .collect(),
    };

    let keys = read_state(
        &intent.intent.state_read,
        db.clone(),
        intent.intent.slots.state.as_slice(),
        &mut data,
        &mut state,
        false,
    )?;

    for StateMutation { address, mutations } in intent.state_mutations {
        let address: Address = address.into();
        for mutation in mutations {
            match mutation {
                Mutation::Key(KeyMutation { key, value }) => {
                    if address == data.this_address {
                        ensure!(
                            keys.iter().any(|k| k.contains(&key)),
                            "Key {:?} must be included in state reads",
                            key
                        );
                    }

                    db.stage(address, key, value);
                }
                Mutation::Range(RangeMutation { key_range, values }) => {
                    if address == data.this_address {
                        ensure!(
                            keys.iter()
                                .any(|k| KeyRangeIter::new(key_range.clone())
                                    .all(|k2| k.contains(&k2))),
                            "Key {:?} must be included in state reads",
                            key_range
                        );
                    }
                    let len = KeyRangeIter::new(key_range.clone()).count();
                    ensure!(
                        len == values.len(),
                        "Key range and values must be the same length"
                    );
                    for (key, value) in KeyRangeIter::new(key_range).zip(values) {
                        db.stage(address, key, value);
                    }
                }
            }
        }
    }

    read_state(
        &intent.intent.state_read,
        db.clone(),
        intent.intent.slots.state.as_slice(),
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
    read: &[Vec<u8>],
    db: Db,
    state_slots: &[StateSlot],
    data: &mut Data,
    state: &mut [Option<u64>],
    delta: bool,
) -> anyhow::Result<Vec<KeyRange>> {
    let mut all_keys = Vec::new();
    if !read.is_empty() {
        let programs: Vec<Vec<StateReadOp>> = read
            .iter()
            .map(|read| serde_json::from_slice(read))
            .collect::<Result<_, _>>()?;
        for slot in state_slots {
            let Some(program) = programs.get(slot.program_index as usize) else {
                bail!("State read program out of bounds");
            };
            let ReadOutput { keys, memory } = vm::read(&db, data, program.clone())?;
            all_keys.extend(keys);
            if memory.len() != slot.amount as usize {
                bail!(
                    "State read failed, read {} words, expected {}",
                    memory.len(),
                    slot.amount
                );
            }
            for (s, r) in state.iter_mut().skip(slot.index as usize).zip(memory) {
                *s = r;
            }
            if delta {
                data.state_delta = state.to_vec();
            } else {
                data.state = state.to_vec();
            }
        }
    }
    Ok(all_keys)
}

fn check_slots(slots: &Slots, solution: &SolutionData) -> anyhow::Result<()> {
    ensure!(slots.decision_variables == solution.decision_variables.len() as u32);
    match (&slots.input_message_args, &solution.input_message) {
        (None, None) => (),
        (None, Some(_)) | (Some(_), None) => bail!("Input message mismatch"),
        (Some(slot_args), Some(solution_args)) => {
            ensure!(slot_args.len() == solution_args.args.len());
            for (expected, args) in slot_args.iter().zip(solution_args.args.iter()) {
                ensure!(*expected == args.len() as u16);
            }
        }
    }
    ensure!(slots.output_messages_args.len() == solution.output_messages.len());
    for (expected, args) in slots
        .output_messages_args
        .iter()
        .zip(solution.output_messages.iter())
    {
        ensure!(expected.len() == args.args.len());
        for (len, got) in expected.iter().zip(args.args.iter()) {
            ensure!(*len == got.len() as u16);
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
    ensure!(
        output.len() == 1,
        "Constraint failed with multiple return values"
    );
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
            stack.push(word2);
            stack.push(word1);
        }
        Op::Pred(pred) => check_predicate(stack, pred)?,
        Op::Alu(alu) => check_alu(stack, alu)?,
        Op::Access(access) => check_access(data, stack, access)?,
        Op::Crypto(crypto) => check_crypto(stack, crypto)?,
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
        Alu::Add => Some(word1 + word2),
        Alu::Sub => Some(word1 - word2),
        Alu::Mul => Some(word1 * word2),
        Alu::Div => Some(word1 / word2),
        Alu::Mod => Some(word1 % word2),
        Alu::HashOffset => {
            let offset = word2;
            let hash3 = word1;
            let hash2 = pop_one(stack)?;
            let hash1 = pop_one(stack)?;
            let hash0 = pop_one(stack)?;
            let hash = [hash0, hash1, hash2, hash3];
            let Some(hash) = add_to_key(hash, 0, offset) else {
                bail!("Hash offset overflow. Hash: {:?}, Offset: {}", hash, offset);
            };
            stack.extend(hash);
            None
        }
    };
    if let Some(result) = result {
        stack.push(result);
    }
    Ok(())
}

fn check_access(data: &Data, stack: &mut Vec<u64>, access: Access) -> anyhow::Result<()> {
    match access {
        Access::DecisionVar => {
            let slot = pop_one(stack)?;
            let slot: usize = slot.try_into()?;
            let Some(var) = data.decision_variables.get(slot) else {
                bail!("{:?} access out of bounds", access);
            };
            stack.push(*var);
        }
        Access::DecisionVarRange => {
            let (slot, range) = pop_two(stack)?;
            let slot: usize = slot.try_into()?;
            let range: usize = range.try_into()?;
            let Some(slice) = data.decision_variables.get(slot..(slot + range)) else {
                bail!("{:?} access out of bounds", access);
            };
            stack.extend(slice);
        }
        Access::State => {
            let (slot, delta) = pop_two(stack)?;
            let slot: usize = slot.try_into()?;
            let state = match delta {
                0 => data.state.get(slot).copied().map(Option::unwrap_or_default),
                1 => data
                    .state_delta
                    .get(slot)
                    .copied()
                    .map(Option::unwrap_or_default),
                _ => bail!("{:?} Invalid state access", access),
            };
            match state {
                Some(state) => {
                    stack.push(state);
                }
                None => bail!("{:?} access out of bounds", access),
            }
        }
        Access::StateRange => {
            let (slot, range) = pop_two(stack)?;
            let slot: usize = slot.try_into()?;
            let range: usize = range.try_into()?;
            match pop_one(stack)? {
                0 => {
                    let iter = data
                        .state
                        .get(slot..(slot + range))
                        .map(|i| i.iter().copied().map(Option::unwrap_or_default));
                    match iter {
                        Some(iter) => {
                            stack.extend(iter);
                        }
                        None => bail!("{:?} access out of bounds", access),
                    }
                }
                1 => {
                    let iter = data
                        .state_delta
                        .get(slot..(slot + range))
                        .map(|i| i.iter().copied().map(Option::unwrap_or_default));
                    match iter {
                        Some(iter) => {
                            stack.extend(iter);
                        }
                        None => bail!("{:?} access out of bounds", access),
                    }
                }
                _ => bail!("{:?} Invalid state access", access),
            }
        }
        Access::StateIsSome => {
            let (address, delta) = pop_two(stack)?;
            let address: usize = address.try_into()?;
            let state = match delta {
                0 => data.state.get(address).copied(),
                1 => data.state_delta.get(address).copied(),
                _ => bail!("{:?} Invalid state access", access),
            };
            match state {
                Some(state) => {
                    stack.push(state.is_some() as u64);
                }
                None => bail!("{:?} access out of bounds", access),
            }
        }
        Access::StateIsSomeRange => {
            let (slot, range) = pop_two(stack)?;
            let slot: usize = slot.try_into()?;
            let range: usize = range.try_into()?;
            match pop_one(stack)? {
                0 => {
                    let iter = data
                        .state
                        .get(slot..(slot + range))
                        .map(|iter| iter.iter().map(|i| i.is_some() as u64));
                    match iter {
                        Some(iter) => {
                            stack.extend(iter);
                        }
                        None => bail!("{:?} access out of bounds", access),
                    }
                }
                1 => {
                    let iter = data
                        .state_delta
                        .get(slot..(slot + range))
                        .map(|iter| iter.iter().map(|i| i.is_some() as u64));
                    match iter {
                        Some(iter) => {
                            stack.extend(iter);
                        }
                        None => bail!("{:?} access out of bounds", access),
                    }
                }
                _ => bail!("{:?} Invalid state access", access),
            }
        }
        Access::InputMsgSenderWord => {
            let index: usize = pop_one(stack)?.try_into()?;
            match data
                .input_message
                .as_ref()
                .and_then(|m| m.sender.get(index))
            {
                Some(word) => {
                    stack.push(*word);
                }
                None => bail!("{:?} access out of bounds", access),
            }
        }
        Access::InputMsgSender => match &data.input_message {
            Some(m) => {
                stack.extend(m.sender);
            }
            None => bail!("{:?} access out of bounds", access),
        },
        Access::InputMsgArgWord => {
            let (arg_index, word_index) = pop_two(stack)?;
            let arg_index: usize = arg_index.try_into()?;
            let word_index: usize = word_index.try_into()?;
            match data
                .input_message
                .as_ref()
                .and_then(|m| m.args.get(arg_index))
                .and_then(|a| a.get(word_index))
            {
                Some(word) => {
                    stack.push(*word);
                }
                None => bail!("{:?} access out of bounds", access),
            }
        }
        Access::InputMsgArgRange => {
            let index = pop_one(stack)?;
            let (start, end) = pop_two(stack)?;
            let index: usize = index.try_into()?;
            let start: usize = start.try_into()?;
            let end: usize = end.try_into()?;
            match data
                .input_message
                .as_ref()
                .and_then(|m| m.args.get(index))
                .and_then(|a| a.get(start..end))
            {
                Some(iter) => {
                    stack.extend(iter);
                }
                None => bail!("{:?} access out of bounds", access),
            }
        }
        Access::InputMsgArg => {
            let index = pop_one(stack)?;
            let index: usize = index.try_into()?;
            match data.input_message.as_ref().and_then(|m| m.args.get(index)) {
                Some(iter) => {
                    let before = stack.len();
                    stack.extend(iter);
                    let after = stack.len();
                    stack.push((after - before).try_into()?);
                }
                None => bail!("{:?} access out of bounds", access),
            }
        }
        Access::OutputMsgArgWord => {
            let (arg_index, word_index) = pop_two(stack)?;
            let msg_index = pop_one(stack)?;
            let msg_index: usize = msg_index.try_into()?;
            let arg_index: usize = arg_index.try_into()?;
            let word_index: usize = word_index.try_into()?;
            let word = data
                .output_messages
                .get(msg_index)
                .and_then(|m| m.args.get(arg_index))
                .and_then(|a| a.get(word_index));
            match word {
                Some(word) => {
                    stack.push(*word);
                }
                None => bail!("{:?} access out of bounds", access),
            }
        }
        Access::OutputMsgArgRange => {
            let (start, end) = pop_two(stack)?;
            let (msg_index, arg_index) = pop_two(stack)?;
            let msg_index: usize = msg_index.try_into()?;
            let arg_index: usize = arg_index.try_into()?;
            let start: usize = start.try_into()?;
            let end: usize = end.try_into()?;
            let iter = data
                .output_messages
                .get(msg_index)
                .and_then(|m| m.args.get(arg_index))
                .and_then(|a| a.get(start..end).map(|iter| iter.iter().copied()));
            match iter {
                Some(iter) => {
                    stack.extend(iter);
                }
                None => bail!("{:?} access out of bounds", access),
            }
        }
        Access::OutputMsgArg => {
            let (msg_index, arg_index) = pop_two(stack)?;
            let msg_index: usize = msg_index.try_into()?;
            let arg_index: usize = arg_index.try_into()?;
            let iter = data
                .output_messages
                .get(msg_index)
                .and_then(|m| m.args.get(arg_index).map(|iter| iter.iter().copied()));
            match iter {
                Some(iter) => {
                    let before = stack.len();
                    stack.extend(iter);
                    let after = stack.len();
                    stack.push((after - before).try_into()?);
                }
                None => bail!("{:?} access out of bounds", access),
            }
        }
    }
    Ok(())
}

fn check_crypto(stack: &mut Vec<u64>, crypto: Crypto) -> anyhow::Result<()> {
    match crypto {
        Crypto::Sha256 => {
            use sha2::Digest;

            let data_length = pop_one(stack)?;
            let Some(data_pos) = stack.len().checked_sub(data_length.try_into()?) else {
                bail!("stack underflow");
            };
            let data = stack
                .drain(data_pos..)
                .flat_map(unpack_bytes)
                .collect::<Vec<_>>();
            let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
            hasher.update(&data);
            let result: [u8; 32] = hasher.finalize().into();
            for word in result.chunks_exact(8).map(pack_bytes) {
                stack.push(word);
            }
        }
        Crypto::VerifyEd25519 => {
            use ed25519_dalek::Signature;
            use ed25519_dalek::Verifier;
            use ed25519_dalek::VerifyingKey;

            let public_key: Vec<u8> = stack.drain(4..).flat_map(unpack_bytes).collect();
            let Ok(public_key): Result<[u8; 32], _> = public_key.try_into() else {
                bail!("Invalid public key")
            };
            let signature: Vec<u8> = stack.drain(8..).flat_map(unpack_bytes).collect();
            let Ok(signature): Result<[u8; 64], _> = signature.try_into() else {
                bail!("Invalid signature")
            };
            let data_length = pop_one(stack)?;
            let Some(data_pos) = stack.len().checked_sub(data_length.try_into()?) else {
                bail!("stack underflow");
            };
            let data: Vec<u8> = stack.drain(data_pos..).flat_map(unpack_bytes).collect();
            let sig = Signature::from_bytes(&signature);
            let pub_key = VerifyingKey::from_bytes(&public_key)?;
            let result = pub_key.verify(&data, &sig).is_ok();
            stack.push(result as u64);
        }
    }
    Ok(())
}

pub fn pack_n_bytes(result: &[u8]) -> Vec<u64> {
    result.chunks(8).map(pack_bytes).collect()
}

pub fn pack_bytes(result: &[u8]) -> u64 {
    let mut out: u64 = 0;
    for (i, byte) in result.iter().rev().enumerate() {
        out |= (*byte as u64) << (i * 8);
    }
    out
}

pub fn unpack_bytes(word: u64) -> [u8; 8] {
    let mut out = [0u8; 8];
    for (i, byte) in out.iter_mut().rev().enumerate() {
        *byte = (word >> (i * 8)) as u8;
    }
    out
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
