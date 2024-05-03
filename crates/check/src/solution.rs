//! Items related to validating `Solution`s.

use crate::{
    state_read_vm::{
        self, Access, BytecodeMapped, Gas, GasLimit, SolutionAccess, StateRead, StateSlotSlice,
        StateSlots,
    },
    types::{
        intent::Intent,
        slots::{self, StateSlot},
        solution::{Solution, SolutionDataIndex},
        IntentAddress, Word,
    },
};
use std::sync::Arc;
use tokio::task::JoinSet;

/// The utility score of a solution.
pub type Utility = f64;

/// Checks a solution against its associated intents.
///
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
/// Returns the `utility` score of the solution alongside the total gas spent.
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
        let Some(intent) = get_intent(&data.intent_to_solve) else {
            anyhow::bail!("Intent in solution data not found in intents set");
        };
        let solution = solution.clone();
        let pre_state: SA = pre_state.clone();
        let post_state: SB = post_state.clone();
        let solution_data_index: SolutionDataIndex = solution_data_index.try_into()?;

        set.spawn(async move {
            // Get the length of state slots for this intent.
            let intent_state_len: usize = slots::state_len(&intent.slots.state)
                .ok_or(anyhow::anyhow!("State slots have no length"))?
                .try_into()?;

            let mut total_gas = 0;

            // Initialize pre and post slots.
            let mut pre_slots: Vec<Option<Word>> = vec![None; intent_state_len];
            let mut post_slots: Vec<Option<Word>> = vec![None; intent_state_len];
            let solution_access = SolutionAccess::new(&solution, solution_data_index);

            // Read pre and post states.
            for (state_read_index, state_read) in intent.state_read.iter().enumerate() {
                let state_read_index: u16 = state_read_index.try_into()?;

                // Map the bytecode ops ahead of execution to share the mapping
                // between both pre and post state slot reads.
                let state_read_mapped = BytecodeMapped::try_from(&state_read[..])?;

                // Read pre state slots.
                let (gas, new_pre_slots) = read_state(
                    &state_read_mapped,
                    Access {
                        solution: solution_access,
                        state_slots: StateSlots {
                            pre: &pre_slots,
                            post: &post_slots,
                        },
                    },
                    &pre_state,
                )
                .await?;
                total_gas += gas;

                // Validate the `new_pre_slots`, then update `pre_slots`.
                write_slots(
                    state_read_index,
                    &intent.slots.state,
                    &mut pre_slots,
                    &new_pre_slots,
                )?;

                // Read post state slots.
                let (gas, new_post_slots) = read_state(
                    &state_read_mapped,
                    Access {
                        solution: solution_access,
                        state_slots: StateSlots {
                            pre: &pre_slots,
                            post: &post_slots,
                        },
                    },
                    &post_state,
                )
                .await?;
                total_gas += gas;
                write_slots(
                    state_read_index,
                    &intent.slots.state,
                    &mut post_slots,
                    &new_post_slots,
                )?;
            }

            // Check constraints.
            let utility: Utility = todo!();
            // let utility = check_constraints(
            //     intent.clone(),
            //     pre_slots,
            //     post_slots,
            //     solution,
            //     solution_data_index,
            // )
            // .await?;
            Ok((utility, total_gas))
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

/// Reads state slots from storage using the given bytecode.
///
/// The result is written to VM's memory.
///
/// Returns the gas spent alongside the state slots consumed from the VM's memory.
async fn read_state<S>(
    bytecode_mapped: &BytecodeMapped<&[u8]>,
    access: Access<'_>,
    state_read: &S,
) -> anyhow::Result<(Gas, Vec<Option<Word>>)>
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

    Ok((gas_spent, vm.into_state_slots()))
}

/// Write to the correct slots based on the state read index.
fn write_slots(
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
