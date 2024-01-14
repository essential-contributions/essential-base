pub mod vm;

#[derive(Debug, Clone, Default)]
pub struct StateSlots(Vec<StateSlot>);

#[derive(Debug, Clone)]
pub struct StateSlot {
    pub index: u64,
    pub amount: u64,
    pub call: VmCall,
}

#[derive(Debug, Clone)]
pub struct VmCall {
    /// Index of bytecode that retrieves the value from the state.
    pub index: u64,
}

impl StateSlots {
    pub fn new(slots: Vec<StateSlot>) -> Self {
        Self(slots)
    }

    pub fn len(&self) -> usize {
        self.0
            .iter()
            .map(|slot| slot.index + slot.amount)
            .max()
            .unwrap_or(0) as usize
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_slice(&self) -> &[StateSlot] {
        &self.0
    }
}
