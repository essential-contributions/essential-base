use anyhow::bail;

use crate::data::Data;
use crate::data::InputMessage;
use crate::data::OutputMessage;
use crate::db::Db;
use crate::op::Access;
use crate::op::Alu;
use crate::op::Op;
use crate::op::Pred;
use crate::state_read;
use crate::state_read::load_module;
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
    pub state_mutations: Vec<(u64, Option<u64>)>,
}

pub fn check(db: &mut Db, intent: SolvedIntent) -> anyhow::Result<u64> {
    let len = intent
        .intent
        .state_slots
        .iter()
        .map(|s| s.index + s.amount)
        .max()
        .unwrap_or(0);
    let mut state = vec![0; len as usize];
    let mut state_delta = vec![0; len as usize];

    let (mut store, module) = load_module(&intent.intent.state_read, db.clone())?;
    for slot in &intent.intent.state_slots {
        let result = state_read::read_state(&mut store, &module, &slot.fn_name, slot.params)?;
        if result.len() != slot.amount as usize {
            bail!("State read failed");
        }
        for (s, r) in state.iter_mut().skip(slot.index as usize).zip(result) {
            *s = r;
        }
    }

    for (key, value) in intent.solution.state_mutations {
        db.stage(key, value);
    }

    let (mut store, module) = load_module(&intent.intent.state_read, db.clone())?;
    for slot in &intent.intent.state_slots {
        let result = state_read::read_state(&mut store, &module, &slot.fn_name, slot.params)?;
        if result.len() != slot.amount as usize {
            bail!("State read failed");
        }
        for (s, r) in state_delta.iter_mut().skip(slot.index as usize).zip(result) {
            *s = r;
        }
    }

    let data = Data {
        decision_variables: intent.solution.decision_variables.clone(),
        state,
        state_delta,
        input_message: intent.solution.input_message.clone(),
        output_messages: intent.solution.output_messages.clone(),
    };

    check_constraints(&data, &intent.intent.constraints)?;

    match intent.intent.directive {
        Directive::Satisfy => Ok(1),
        Directive::Maximize(code) | Directive::Minimize(code) => {
            let ops = serde_json::from_slice(&code)?;
            eval(&data, ops)
        }
    }
}

fn check_constraints(data: &Data, constraints: &Vec<Vec<u8>>) -> anyhow::Result<()> {
    for constraint in constraints {
        let ops = serde_json::from_slice(constraint)?;
        check_constraint(data, ops)?;
    }

    Ok(())
}

fn check_constraint(data: &Data, ops: Vec<Op>) -> anyhow::Result<()> {
    let output = eval(data, ops)?;

    if output != 1 {
        anyhow::bail!("Constraint failed");
    }

    Ok(())
}

fn eval(data: &Data, ops: Vec<Op>) -> anyhow::Result<u64> {
    let mut stack = Vec::new();

    for op in ops {
        match op {
            Op::Push(word) => {
                stack.push(word);
            }
            Op::Pop => {
                stack.pop();
            }
            Op::Dup => {
                let word = pop_one(&mut stack)?;
                stack.push(word);
                stack.push(word);
            }
            Op::Swap => {
                let (word1, word2) = pop_two(&mut stack)?;
                stack.push(word1);
                stack.push(word2);
            }
            Op::Pred(pred) => check_predicate(&mut stack, pred)?,
            Op::Alu(alu) => check_alu(&mut stack, alu)?,
            Op::Access(access) => check_access(data, &mut stack, access)?,
        }
        println!("Op: {:?}, Stack: {:?}", op, stack);
    }

    println!("Result: {:?}", stack);

    pop_one(&mut stack)
}

fn check_predicate(stack: &mut Vec<u64>, pred: Pred) -> anyhow::Result<()> {
    let word1 = pop_one(stack)?;
    let result = match pred {
        Pred::Eq => word1 == pop_one(stack)?,
        Pred::Gt => word1 > pop_one(stack)?,
        Pred::Lt => word1 < pop_one(stack)?,
        Pred::Gte => word1 >= pop_one(stack)?,
        Pred::Lte => word1 <= pop_one(stack)?,
        Pred::And => word1 != 0 && pop_one(stack)? != 0,
        Pred::Or => word1 != 0 || pop_one(stack)? != 0,
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
            let var = data.decision_variables[address as usize];
            stack.push(var);
        }
        Access::DecisionVarRange => {
            let (index, range) = pop_two(stack)?;
            stack.extend(
                data.decision_variables[index as usize..(index + range) as usize]
                    .iter()
                    .copied(),
            );
        }
        Access::State => {
            let (address, delta) = pop_two(stack)?;
            let state = match delta {
                0 => data.state[address as usize],
                1 => data.state_delta[address as usize],
                _ => anyhow::bail!("Invalid state access"),
            };
            stack.push(state);
        }
        Access::StateRange => {
            let (address, range) = pop_two(stack)?;
            match pop_one(stack)? {
                0 => {
                    let iter = &data.state[address as usize..(address + range) as usize];
                    stack.extend(iter);
                }
                1 => {
                    let iter = &data.state_delta[address as usize..(address + range) as usize];
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

fn pop_one(stack: &mut Vec<u64>) -> anyhow::Result<u64> {
    stack
        .pop()
        .ok_or_else(|| anyhow::anyhow!("Stack underflow"))
}

fn pop_two(stack: &mut Vec<u64>) -> anyhow::Result<(u64, u64)> {
    let word1 = pop_one(stack)?;
    let word2 = pop_one(stack)?;
    Ok((word2, word1))
}
