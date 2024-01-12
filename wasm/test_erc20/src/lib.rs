#[no_mangle]
pub fn get_sender_bal(args_ptr: i32, args_len: i32) -> i32 {
    let args = guest_sdk::decode_args(args_ptr, args_len);
    let key = guest_sdk::hash(args[0].clone());
    let balance = guest_sdk::state_read_word_range(key, 1);
    guest_sdk::encode_result(balance)
}

#[no_mangle]
pub fn get_receiver_bal(args_ptr: i32, args_len: i32) -> i32 {
    let args = guest_sdk::decode_args(args_ptr, args_len);
    let key = guest_sdk::hash(args[0].clone());
    let balance = guest_sdk::state_read_word_range(key, 1);
    guest_sdk::encode_result(balance)
}
