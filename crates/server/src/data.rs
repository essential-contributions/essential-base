#[derive(Clone, Debug, Default)]
pub struct Data {
    pub decision_variables: Vec<u64>,
    pub state: Vec<Option<u64>>,
    pub state_delta: Vec<Option<u64>>,
    pub input_message: InputMessage,
    pub output_messages: Vec<OutputMessage>,
}

#[derive(Clone, Debug, Default)]
pub struct InputMessage {
    pub sender: [u64; 8],
    pub args: Vec<Vec<u64>>,
}

#[derive(Clone, Debug, Default)]
pub struct OutputMessage {
    pub recipient: [u64; 8],
    pub args: Vec<Vec<u64>>,
}
