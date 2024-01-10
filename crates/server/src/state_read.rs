use anyhow::bail;
use wasmtime::*;

use crate::db::Db;

pub struct Slot {
    pub index: u64,
    pub amount: u64,
    pub fn_name: String,
    pub params: (),
}

pub fn load_module(bytes: &[u8], db: Db) -> anyhow::Result<(Store<Db>, Instance)> {
    let engine = Engine::default();
    let module = Module::from_binary(&engine, bytes)?;
    let mut store = Store::new(&engine, db);
    // Host function that the guest calls to read state.
    let state_read_word_range = Func::wrap(
        &mut store,
        |mut caller: Caller<'_, Db>, key: u64, amount: i32, buf_ptr: i32| -> anyhow::Result<i32> {
            // Get the guest memory.
            let Some(Extern::Memory(mem)) = caller.get_export("memory") else {
                bail!("failed to find host memory");
            };

            // Get the data from the database at the given key and amount.
            let result: Vec<u8> = caller
                .data()
                .read_range(&key, amount)
                .iter()
                .flat_map(|i| i.to_le_bytes())
                .collect();

            // Write the result to the guest memory at the given location.
            mem.write(&mut caller, buf_ptr as usize, &result)?;

            // Return the length of the result.
            Ok((result.len() / 8) as i32)
        },
    );

    // Instantiate the module with the host function.
    let imports = [state_read_word_range.into()];
    let instance = Instance::new(&mut store, &module, &imports)?;
    Ok((store, instance))
}

pub fn read_state(
    mut store: &mut Store<Db>,
    instance: &Instance,
    fn_name: &str,
    params: (),
) -> anyhow::Result<Vec<u64>> {
    // Run the wasm.
    let get_state = instance.get_typed_func::<(), i32>(&mut store, fn_name)?;

    let ptr = get_state.call(&mut store, params)?;

    // Get the guest memory.
    let mem = instance.get_memory(&mut store, "memory").unwrap();

    let size = std::mem::size_of::<[i32; 2]>();
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

    let result_len = result_len * 8;

    // Get the result from the guest memory.
    let Some(output) = mem
        .data(store)
        .get(result_ptr as usize..(result_ptr as usize + result_len as usize))
    else {
        bail!("failed to get result output");
    };
    Ok(output
        .chunks_exact(8)
        .map(|i| {
            u64::from_le_bytes(
                i.try_into()
                    .expect("Can't fail as we know the size of the chunk"),
            )
        })
        .collect())
}
