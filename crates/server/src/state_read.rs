pub use wasm::*;

pub mod vm;
pub mod wasm;

#[derive(Debug, Clone)]
pub struct StateSlot<Call> {
    pub index: u64,
    pub amount: u64,
    pub call: Call,
}

#[derive(Debug, Clone)]
pub struct WasmCall {
    pub fn_name: String,
    pub params: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct VmCall {
    /// Index of bytecode that retrieves the value from the state.
    pub index: u64,
}

#[derive(Debug, Clone)]
pub enum StateRead<W, V> {
    Wasm(W),
    Vm(V),
}

impl<W: Default, V: Default> Default for StateRead<W, V> {
    fn default() -> Self {
        Self::Wasm(W::default())
    }
}

impl StateRead<Vec<StateSlot<WasmCall>>, Vec<StateSlot<VmCall>>> {
    pub fn len(&self) -> usize {
        match self {
            Self::Wasm(wasm) => wasm
                .iter()
                .map(|slot| slot.index + slot.amount)
                .max()
                .unwrap_or(0) as usize,
            Self::Vm(vm) => vm
                .iter()
                .map(|slot| slot.index + slot.amount)
                .max()
                .unwrap_or(0) as usize,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
