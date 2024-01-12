#[no_mangle]
pub fn foo(_args_ptr: i32, _args_len: i32) -> i32 {
    let data = guest_sdk::state_read_word_range(0, 10);
    let mut data = data
        .into_iter()
        .filter(|i| i.map_or(false, |i| i % 2 == 0))
        .take(4)
        .collect::<Vec<_>>();
    data.sort();
    guest_sdk::encode_result(data)
}

#[no_mangle]
pub fn bar(args_ptr: i32, args_len: i32) -> i32 {
    let args = guest_sdk::decode_args(args_ptr, args_len);
    let mut data = guest_sdk::state_read_word_range(10, 5);
    for d in data.iter_mut().flatten() {
        *d *= args[0][0]
    }
    data.sort();
    guest_sdk::encode_result(data)
}

#[no_mangle]
pub fn hash(args_ptr: i32, args_len: i32) -> i32 {
    let args = guest_sdk::decode_args(args_ptr, args_len);
    let hash = guest_sdk::hash(args[0].clone());
    guest_sdk::encode_result(hash.into_iter().map(Some).collect())
}
