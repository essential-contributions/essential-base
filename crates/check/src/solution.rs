//! Items related to validating `Solution`s.

use crate::{
    constraint_vm,
    state_read_vm::{
        self, Access, BytecodeMapped, Gas, GasLimit, SolutionAccess, StateRead, StateSlotSlice,
        StateSlots,
    },
    types::{
        intent::{Directive, Intent},
        slots::{self, StateSlot},
        solution::{Solution, SolutionDataIndex},
        IntentAddress, Word,
    },
};
use std::sync::Arc;
use tokio::task::JoinSet;

/// The utility score of a solution.
pub type Utility = f64;

/// Checks a solution against its associated intents, one task per solution data.
///
/// For each of the solution's `data` elements, reads the pre and post state
/// slots for the associated intent with access to the given `pre_state` and
/// `post_state`, then checks all constraints over the resulting pre and post
/// state slots.
///
/// ## Arguments
///
/// - `pre_state` must provide access to state *prior to* mutations being applied.
/// - `post_state` must provide access to state *post* mutations being applied.
/// - `get_intent` provides immediate access to an intent associated with the given
///   solution. Calls to `intent` must complete immediately. The necessary
///   intents are assumed to have been read from storage ahead of time.
///
/// Returns the utility score of the solution alongside the total gas spent.
pub async fn check_intents<SA, SB>(
    pre_state: &SA,
    post_state: &SB,
    solution: Arc<Solution>,
    get_intent: impl Fn(&IntentAddress) -> Option<Arc<Intent>>,
) -> anyhow::Result<(Utility, Gas)>
where
    SA: Clone + StateRead + Send + Sync + 'static,
    SB: Clone + StateRead + Send + Sync + 'static,
    SA::Future: Send,
    SB::Future: Send,
{
    // Read pre and post states then check constraints.
    let mut set: JoinSet<anyhow::Result<_>> = JoinSet::new();
    for (solution_data_index, data) in solution.data.iter().enumerate() {
        let solution_data_index: SolutionDataIndex = solution_data_index.try_into()?;
        let Some(intent) = get_intent(&data.intent_to_solve) else {
            anyhow::bail!("Intent in solution data not found in intents set");
        };
        let solution = solution.clone();
        let pre_state: SA = pre_state.clone();
        let post_state: SB = post_state.clone();
        set.spawn(async move {
            let pre_state = pre_state;
            let post_state = post_state;
            check_intent(
                &pre_state,
                &post_state,
                solution,
                intent,
                solution_data_index,
            )
            .await
        });
    }

    // Calculate total utility and gas used.
    // TODO: Gas is only calculated for state reads.
    // Add gas tracking for constraint checking.
    let mut total_gas: u64 = 0;
    let mut utility: f64 = 0.0;
    while let Some(res) = set.join_next().await {
        let (u, g) = res??;
        utility += u;

        // Ensure utility does not overflow.
        anyhow::ensure!(utility != f64::INFINITY, "Utility overflow");

        total_gas = total_gas
            .checked_add(g)
            .ok_or(anyhow::anyhow!("Gas overflow"))?;
    }

    Ok((utility, total_gas))
}

/// Checks a solution against a single intent using the solution data at the given index.
///
/// Reads all pre and post state slots into memory, then checks all constraints.
///
/// ## Arguments
///
/// - `pre_state` must provide access to state *prior to* mutations being applied.
/// - `post_state` must provide access to state *post* mutations being applied.
/// - `solution_data_index` represents the data within `solution.data` that claims
///   to solve this intent.
///
/// Returns the utility score of the solution alongside the total gas spent.
pub async fn check_intent<SA, SB>(
    pre_state: &SA,
    post_state: &SB,
    solution: Arc<Solution>,
    intent: Arc<Intent>,
    solution_data_index: SolutionDataIndex,
) -> anyhow::Result<(Utility, Gas)>
where
    SA: StateRead + Sync,
    SB: StateRead + Sync,
{
    // Get the length of state slots for this intent.
    let intent_state_len: usize = slots::state_len(&intent.slots.state)
        .ok_or(anyhow::anyhow!("State slots have no length"))?
        .try_into()?;

    // Track the total gas spent over all execution.
    let mut total_gas = 0;

    // Initialize pre and post slots. These will contain all state slots for all state reads.
    let mut pre_slots: Vec<Option<Word>> = vec![None; intent_state_len];
    let mut post_slots: Vec<Option<Word>> = vec![None; intent_state_len];
    let solution_access = SolutionAccess::new(&solution, solution_data_index);

    // Read pre and post states.
    for (state_read_index, state_read) in intent.state_read.iter().enumerate() {
        let state_read_index: u16 = state_read_index.try_into()?;

        // Map the bytecode ops ahead of execution to share the mapping
        // between both pre and post state slot reads.
        let state_read_mapped = BytecodeMapped::try_from(&state_read[..])?;

        // Read pre state slots and write them to the pre_slots slice.
        let (gas, new_pre_slots) = read_state_slots(
            &state_read_mapped,
            Access {
                solution: solution_access,
                state_slots: StateSlots {
                    pre: &pre_slots,
                    post: &post_slots,
                },
            },
            pre_state,
        )
        .await?;
        total_gas += gas;
        write_state_slots(
            state_read_index,
            &intent.slots.state,
            &mut pre_slots,
            &new_pre_slots,
        )?;

        // Read post state slots and write them to the post_slots slice.
        let (gas, new_post_slots) = read_state_slots(
            &state_read_mapped,
            Access {
                solution: solution_access,
                state_slots: StateSlots {
                    pre: &pre_slots,
                    post: &post_slots,
                },
            },
            post_state,
        )
        .await?;
        total_gas += gas;
        write_state_slots(
            state_read_index,
            &intent.slots.state,
            &mut post_slots,
            &new_post_slots,
        )?;
    }

    // Check constraints.
    let utility = check_intent_constraints(
        solution,
        solution_data_index,
        intent.clone(),
        Arc::from(pre_slots.into_boxed_slice()),
        Arc::from(post_slots.into_boxed_slice()),
    )
    .await?;

    Ok((utility, total_gas))
}

/// Reads state slots from storage using the given bytecode.
///
/// The result is written to VM's memory.
///
/// Returns the gas spent alongside the state slots consumed from the VM's memory.
async fn read_state_slots<S>(
    bytecode_mapped: &BytecodeMapped<&[u8]>,
    access: Access<'_>,
    state_read: &S,
) -> anyhow::Result<(Gas, Box<StateSlotSlice>)>
where
    S: StateRead,
{
    // Create a new state read VM.
    let mut vm = state_read_vm::Vm::default();

    // Read the state into the VM's memory.
    let gas_spent = vm
        .exec_bytecode(
            bytecode_mapped,
            access,
            state_read,
            &|_: &state_read_vm::asm::Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .map_err(|e| anyhow::anyhow!("State read VM execution failed: {e}"))?;

    Ok((gas_spent, vm.into_state_slots().into_boxed_slice()))
}

/// Write to the correct slots based on the state read index.
fn write_state_slots(
    state_read_index: u16,
    state_slots: &[StateSlot],
    slots: &mut StateSlotSlice,
    output_slots: &StateSlotSlice,
) -> anyhow::Result<()> {
    // Find the correct state slot based matching the state read index
    // with the program index.
    let Some(slots) = state_slots
        .iter()
        .find(|slot| slot.program_index == state_read_index)
        .and_then(|slot| {
            let start: usize = slot.index.try_into().ok()?;
            let end: usize = slot.amount.try_into().ok()?;
            let end = end.checked_add(start)?;
            slots.get_mut(start..end)
        })
    else {
        anyhow::bail!("State slot not found for state read program");
    };

    // The length of the output slots must match the length of the slots
    // that are being written to.
    anyhow::ensure!(
        slots.len() == output_slots.len(),
        "State slot length mismatch"
    );

    // Write the output slots to the correct position in the slots.
    slots.copy_from_slice(output_slots);

    Ok(())
}

/// Checks if the given solution data at the given index satisfies the
/// constraints of the given intent.
///
/// Returns the utility of the solution for the given intent.
pub async fn check_intent_constraints(
    solution: Arc<Solution>,
    solution_data_index: SolutionDataIndex,
    intent: Arc<Intent>,
    pre_slots: Arc<StateSlotSlice>,
    post_slots: Arc<StateSlotSlice>,
) -> anyhow::Result<Utility> {
    check_intent_constraints_parallel(
        solution.clone(),
        solution_data_index,
        intent.clone(),
        pre_slots.clone(),
        post_slots.clone(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Constraint VM execution failed: {e}"))?;
    calculate_utility(
        solution,
        solution_data_index,
        intent.clone(),
        pre_slots,
        post_slots,
    )
    .await
}

/// Check intents in parallel without sleeping any threads.
async fn check_intent_constraints_parallel(
    solution: Arc<Solution>,
    solution_data_index: SolutionDataIndex,
    intent: Arc<Intent>,
    pre_slots: Arc<StateSlotSlice>,
    post_slots: Arc<StateSlotSlice>,
) -> anyhow::Result<()> {
    let mut handles = Vec::with_capacity(intent.constraints.len());

    // Spawn each constraint onto a rayon thread and
    // check them in parallel.
    for i in 0..intent.constraints.len() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        handles.push(rx);

        // These are all cheap Arc clones.
        let solution = solution.clone();
        let pre_slots = pre_slots.clone();
        let post_slots = post_slots.clone();
        let intent = intent.clone();

        // Spawn this sync code onto a rayon thread.
        // This is a non-blocking operation.
        rayon::spawn(move || {
            let solution_access = SolutionAccess::new(&solution, solution_data_index);
            let access = Access {
                solution: solution_access,
                state_slots: StateSlots {
                    pre: &pre_slots,
                    post: &post_slots,
                },
            };
            let res = constraint_vm::eval_bytecode_iter(
                intent
                    .constraints
                    .get(i)
                    .expect("Safe due to above len check")
                    .iter()
                    .copied(),
                access,
            );
            // Send the result back to the main thread.
            // Send errors are ignored as if the recv is gone there's no one to send to.
            let _ = tx.send((i, res));
        })
    }

    // There's no way to know the size of these.
    let mut failed = Vec::new();
    let mut unsatisfied = Vec::new();

    // Wait for all constraints to finish.
    // The order of waiting on handles is not important as all
    // constraints make progress independently.
    for handle in handles {
        // Get the index and result from the handle.
        let (i, res): (usize, Result<bool, _>) = handle.await?;
        match res {
            // If the constraint failed, add it to the failed list.
            Err(err) => failed.push((i, err)),
            // If the constraint was unsatisfied, add it to the unsatisfied list.
            Ok(b) if !b => unsatisfied.push(i),
            // Otherwise, the constraint was satisfied.
            _ => (),
        }
    }

    // If there are any failed constraints, return an error.
    if !failed.is_empty() {
        return Err(essential_constraint_vm::error::ConstraintErrors(failed).into());
    }

    // If there are any unsatisfied constraints, return an error.
    if !unsatisfied.is_empty() {
        return Err(essential_constraint_vm::error::ConstraintsUnsatisfied(unsatisfied).into());
    }
    Ok(())
}

/// Calculates utility of solution for intent.
///
/// Returns utility.
async fn calculate_utility(
    solution: Arc<Solution>,
    solution_data_index: SolutionDataIndex,
    intent: Arc<Intent>,
    pre_slots: Arc<StateSlotSlice>,
    post_slots: Arc<StateSlotSlice>,
) -> anyhow::Result<f64> {
    match &intent.directive {
        Directive::Satisfy => Ok(1.0),
        Directive::Maximize(_) | Directive::Minimize(_) => {
            // Spawn this sync code onto a rayon thread.
            let (tx, rx) = tokio::sync::oneshot::channel();
            rayon::spawn(move || {
                let solution_access = SolutionAccess::new(&solution, solution_data_index);
                let access = Access {
                    solution: solution_access,
                    state_slots: StateSlots {
                        pre: &pre_slots,
                        post: &post_slots,
                    },
                };
                // Extract the directive code.
                let code = match intent.directive {
                    Directive::Maximize(ref code) | Directive::Minimize(ref code) => code,
                    _ => unreachable!("As this is already checked above"),
                };

                // Execute the directive code.
                match constraint_vm::exec_bytecode_iter(code.iter().copied(), access) {
                    Ok(mut stack) => match stack.pop3() {
                        Ok([start, end, value]) => {
                            // Return the normalized value back to the main thread.
                            // Send errors are ignored as if the recv is dropped.
                            let _ = tx.send(normalize_utility(value, start, end));
                        }
                        Err(e) => {
                            // Return the error back to the main thread.
                            // Send errors are ignored as if the recv is dropped.
                            let _ = tx.send(Err(e.into()));
                        }
                    },
                    Err(e) => {
                        // Return the error back to the main thread.
                        // Send errors are ignored as if the recv is dropped.
                        let _ = tx.send(Err(e.into()));
                    }
                }
            });

            // Await the result of the utility calculation.
            rx.await?
        }
    }
}

fn normalize_utility(value: Word, start: Word, end: Word) -> anyhow::Result<Utility> {
    anyhow::ensure!(start < end, "Invalid range for directive");
    let normalized = (value - start) as f64 / (end - start) as f64;
    Ok(normalized.clamp(0.0, 1.0))
}
