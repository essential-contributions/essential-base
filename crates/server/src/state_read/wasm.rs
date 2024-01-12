use std::hash::Hash;
use std::hash::Hasher;

use anyhow::bail;
use anyhow::ensure;
use wasmtime::*;

use crate::db::Db;

pub fn load_module(bytes: &[u8], db: Db) -> anyhow::Result<(Store<Db>, Instance)> {
    let engine = Engine::default();
    let module = Module::from_binary(&engine, bytes)?;
    let mut store = Store::new(&engine, db);
    // Host function that the guest calls to read state.
    let state_read_word_range = Func::wrap(
        &mut store,
        |mut caller: Caller<'_, Db>,
         key0: u64,
         key1: u64,
         key2: u64,
         key3: u64,
         amount: i32,
         buf_ptr: i32|
         -> anyhow::Result<i32> {
            // Get the guest memory.
            let Some(Extern::Memory(mem)) = caller.get_export("memory") else {
                bail!("failed to find host memory");
            };

            let key = [key0, key1, key2, key3];
            // Get the data from the database at the given key and amount.
            let result = caller.data().read_range(&key, amount);

            // Encode bit vector of which values are Some.
            let set: bitvec::vec::BitVec<u8, bitvec::order::Msb0> =
                result.iter().map(|i| i.is_some()).collect();
            let set: Vec<u8> = set.into_vec();

            // Encode just the some values.
            let result: Vec<u8> = result
                .iter()
                .flatten()
                .flat_map(|i| i.to_le_bytes())
                .collect();

            // Write the result to the guest memory at the given location.
            mem.write(&mut caller, buf_ptr as usize, &result)?;
            // Write the bit vector of some values to the guest memory after the result.
            mem.write(&mut caller, (buf_ptr as usize) + result.len(), &set)?;

            // Return the length of the result.
            Ok((result.len() / 8) as i32)
        },
    );

    // Host function that the guest calls to hash.
    let hash = Func::wrap(
        &mut store,
        |mut caller: Caller<'_, Db>,
         data_ptr: i32,
         data_len: i32,
         hash_ptr: i32|
         -> anyhow::Result<()> {
            // Get the guest memory.
            let Some(Extern::Memory(mem)) = caller.get_export("memory") else {
                bail!("failed to find host memory");
            };

            let data = mem
                .data(&mut caller)
                .get(data_ptr as usize..(data_ptr as usize + (data_len * 8) as usize))
                .ok_or_else(|| anyhow::anyhow!("failed to get data from guest memory"))?;

            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            let data: Vec<_> = data
                .chunks_exact(8)
                .map(|chunk| {
                    u64::from_le_bytes(
                        chunk
                            .try_into()
                            .expect("Can't fail as we know the size of the chunk."),
                    )
                })
                .collect();
            data.hash(&mut hasher);
            let hash = hasher.finish();
            let hash = [hash, hash, hash, hash];
            let hash = hash
                .iter()
                .flat_map(|i| i.to_le_bytes())
                .collect::<Vec<_>>();

            mem.write(&mut caller, hash_ptr as usize, &hash)?;

            Ok(())
        },
    );
    // Instantiate the module with the host function.
    let imports = [hash.into(), state_read_word_range.into()];
    let instance = Instance::new(&mut store, &module, &imports)?;
    Ok((store, instance))
}

fn write_input_args(
    mut store: &mut Store<Db>,
    instance: &Instance,
    params: Vec<Vec<u64>>,
) -> anyhow::Result<(i32, i32)> {
    // Get the guest memory.
    let Some(mem) = instance.get_memory(&mut store, "memory") else {
        bail!("failed to find guest memory");
    };

    // Get some space from the end of memory.
    // This is hacky.
    let space_needed = params.iter().map(|i| i.len()).sum::<usize>() * 8;
    let len = mem.data(&mut store).len();
    // This is the beginning place to write the params.
    let mut ptr = len - space_needed - 1 - (params.len() * 4);

    // Encode the length of each param.
    let lens = params
        .iter()
        .flat_map(|i| (i.len() as i32).to_le_bytes())
        .collect::<Vec<_>>();

    // Get the starting pointer.
    let start = ptr as i32;
    // Get the number of params.
    let params_len = params.len() as i32;

    // Check that there is enough space in memory.
    ensure!(ptr > 0, "not enough space in memory");

    // Write the param lengths to guest memory.
    mem.write(&mut store, ptr, &lens)?;
    // Move the pointer forward.
    ptr += lens.len();

    // Write each param to guest memory.
    for param in params {
        let param = param
            .into_iter()
            .flat_map(|i| i.to_le_bytes())
            .collect::<Vec<u8>>();
        let len = param.len();
        mem.write(&mut store, ptr, &param)?;
        ptr += len;
    }
    Ok((start, params_len))
}

pub fn read_state(
    mut store: &mut Store<Db>,
    instance: &Instance,
    fn_name: &str,
    params: Vec<Vec<u64>>,
) -> anyhow::Result<Vec<Option<u64>>> {
    // Run the wasm.
    let get_state = instance.get_typed_func::<(i32, i32), i32>(&mut store, fn_name)?;

    // Write the input args to the guest memory.
    let (start, params_len) = write_input_args(store, instance, params)?;

    let ptr = get_state.call(&mut store, (start, params_len))?;

    // Get the guest memory.
    let Some(mem) = instance.get_memory(&mut store, "memory") else {
        bail!("failed to find guest memory");
    };

    let size = std::mem::size_of::<[i32; 4]>();
    // Get the result ptr and length from the guest memory.
    let Some(output) = mem
        .data(&mut store)
        .get(ptr as usize..(ptr as usize + size))
    else {
        bail!("failed to get ptr output");
    };

    // Decode the result ptr and length.
    let output: Vec<i32> = output
        .chunks_exact(4)
        .map(|chunk| {
            i32::from_le_bytes(
                chunk
                    .try_into()
                    .expect("Can't fail as we know the size of the chunk."),
            )
        })
        .collect();

    let Some(&result_ptr) = output.first() else {
        bail!("failed to get result ptr");
    };
    let Some(&result_len) = output.get(1) else {
        bail!("failed to get result len");
    };
    let Some(&set_ptr) = output.get(2) else {
        bail!("failed to get set ptr");
    };
    let Some(&set_truncate_len) = output.get(3) else {
        bail!("failed to get set truncate len");
    };

    // Calculate the number of bytes that the bit vector of somes should be.
    let set_len = set_truncate_len / 8 + if set_truncate_len % 8 == 0 { 0 } else { 1 };

    // Calculate the number of bytes that the result should be.
    let result_len = result_len * 8;

    // Get the result from the guest memory.
    let Some(output) = mem
        .data(&store)
        .get(result_ptr as usize..(result_ptr as usize + result_len as usize))
    else {
        bail!("failed to get result output");
    };

    // Get the bit vector from the guest memory.
    let Some(set) = mem
        .data(&store)
        .get(set_ptr as usize..(set_ptr as usize + set_len as usize))
    else {
        bail!("failed to get result output");
    };
    let set = set.to_vec();

    // Decode from bytes to bit vector.
    let mut set: bitvec::vec::BitVec<u8, bitvec::order::Msb0> = bitvec::vec::BitVec::from_vec(set);

    // Truncate the bit vector to the correct length.
    set.truncate(set_truncate_len as usize);

    let mut iter = output.chunks_exact(8).map(|i| {
        u64::from_le_bytes(
            i.try_into()
                .expect("Can't fail as we know the size of the chunk"),
        )
    });

    // Return Some values where the bit vector is true.
    Ok(set
        .iter()
        .map(|i| if *i { iter.next() } else { None })
        .collect())
}
