extern "C" {
    fn _state_read_word_range(key: u64, amount: i32, buf_ptr: i32) -> i32;
}

pub fn state_read_word_range(key: u64, amount: i32) -> Vec<u64> {
    // Call the host and read state.
    let buf: Vec<u64> = Vec::with_capacity(std::mem::size_of::<u64>() * amount as usize);
    let buf_ptr = buf.leak().as_ptr() as i32;
    let len = unsafe { _state_read_word_range(key, amount, buf_ptr) };
    assert!(len as usize <= std::mem::size_of::<u64>() * amount as usize);
    unsafe { Vec::from_raw_parts(buf_ptr as *mut u64, len as usize, len as usize) }
}

pub fn encode_result(result: Vec<u64>) -> i32 {
    // Get the result length.
    let result_len = result.len() as i32;
    // Leak the result so it isn't dropped.
    let result_ptr = result.leak().as_ptr() as i32;

    // Put the result pointer and length onto the heap.
    let output = Box::new([result_ptr, result_len]);
    // Leak the output so it isn't dropped.
    let output_ptr = Box::leak(output) as *const [i32; 2] as i32;
    output_ptr
}
