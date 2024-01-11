extern "C" {
    fn _state_read_word_range(key: u64, amount: i32, buf_ptr: i32) -> i32;
}

pub fn state_read_word_range(key: u64, amount: i32) -> Vec<Option<u64>> {
    // Create a buffer to read the state into.
    let buf: Vec<u64> = Vec::with_capacity(amount as usize);
    // Leak the buffer so it isn't dropped.
    let buf_ptr = buf.leak().as_ptr() as i32;

    // Calculate the number of bytes that the bit vector of somes should be.
    let bit_vec_len = amount as usize / 8 + if amount as usize % 8 == 0 { 0 } else { 1 };

    // Call the host and read state.
    let len = unsafe { _state_read_word_range(key, amount, buf_ptr) };

    // Get the results from memory.
    let buf_ptr = buf_ptr as *mut u64;
    let some_vals = unsafe { Vec::from_raw_parts(buf_ptr, len as usize, len as usize) };

    // Calculate the ptr that is after the results.
    let set_ptr = unsafe { buf_ptr.offset(len as isize) as *mut u8 };
    // Get the bit vector from memory.
    let set = unsafe { Vec::from_raw_parts(set_ptr, bit_vec_len, bit_vec_len) };

    // Decode the bit vector from bytes.
    let mut set: bitvec::prelude::BitVec<u8, bitvec::order::Msb0> =
        bitvec::vec::BitVec::from_vec(set);
    // Truncate the bit vector to the correct length.
    set.truncate(amount as usize);

    // Return some values if the bit vector is true.
    let mut iter = some_vals.into_iter();
    set.iter()
        .map(|i| if *i { iter.next() } else { None })
        .collect()
}

pub fn encode_result(result: Vec<Option<u64>>) -> i32 {
    // Create a bit vector of somes values.
    let set: bitvec::vec::BitVec<u8, bitvec::order::Msb0> =
        result.iter().map(|i| i.is_some()).collect();
    // Encode the bit vector to bytes.
    let set: Vec<u8> = set.into_vec();

    // Get the actual length of the results (including the Nones).
    let set_len = result.len() as i32;

    // Flatten out the Nones.
    let result: Vec<u64> = result.into_iter().flatten().collect();

    // Get the some result length.
    let result_len = result.len() as i32;
    // Leak the result so it isn't dropped.
    let result_ptr = result.leak().as_ptr() as i32;

    let set_ptr = set.leak().as_ptr() as i32;

    // Put the result pointer and length onto the heap.
    let output = Box::new([result_ptr, result_len, set_ptr, set_len]);
    // Leak the output so it isn't dropped.
    let output_ptr = Box::leak(output) as *const [i32; 4] as i32;
    output_ptr
}
