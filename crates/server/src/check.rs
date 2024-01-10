use anyhow::bail;

use crate::data::Data;
use crate::data::InputMessage;
use crate::data::OutputMessage;
use crate::db::Db;
use crate::state_read;
use crate::state_read::load_module;
use crate::Intent;

pub struct SolvedIntent {
    pub intent: Intent,
    pub solution: Solution,
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
        dbg!(&result);
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

    check_constraints(data, intent.intent.constraints)
}

fn check_constraints(data: Data, constraints: Vec<Vec<u8>>) -> anyhow::Result<u64> {
    todo!()
}