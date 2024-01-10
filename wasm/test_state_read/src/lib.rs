#[no_mangle]
pub fn foo() -> i32 {
    let data = guest_sdk::state_read_word_range(0, 10);
    let mut data = data
        .into_iter()
        .filter(|i| i % 2 == 0)
        .take(4)
        .collect::<Vec<_>>();
    data.sort();
    guest_sdk::encode_result(data)
}

#[no_mangle]
pub fn bar() -> i32 {
    let mut data = guest_sdk::state_read_word_range(10, 5);
    data.sort();
    guest_sdk::encode_result(data)
}
