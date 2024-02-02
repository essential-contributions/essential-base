use crate::db::Address;

pub use essential_types::slots::Slots;

#[derive(Clone, Debug, Default)]
pub struct Data {
    pub this_address: Address,
    pub decision_variables: Vec<u64>,
    pub state: Vec<Option<u64>>,
    pub state_delta: Vec<Option<u64>>,
    pub input_message: Option<InputMessage>,
    pub output_messages: Vec<OutputMessage>,
}

#[derive(Clone, Debug, Default)]
pub struct InputMessage {
    pub sender: Address,
    pub recipient: Address,
    pub args: Vec<Vec<u64>>,
}

#[derive(Clone, Debug, Default)]
pub struct OutputMessage {
    pub args: Vec<Vec<u64>>,
}

impl From<essential_types::solution::InputMessage> for InputMessage {
    fn from(input_message: essential_types::solution::InputMessage) -> Self {
        Self {
            sender: input_message.sender.into(),
            recipient: input_message.recipient.into(),
            args: input_message.args,
        }
    }
}

impl From<essential_types::solution::OutputMessage> for OutputMessage {
    fn from(output_message: essential_types::solution::OutputMessage) -> Self {
        Self {
            args: output_message.args,
        }
    }
}
