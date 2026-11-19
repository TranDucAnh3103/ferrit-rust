use crate::sandbox::{HostState, PluginError};
use wasmtime::{Caller, Linker, Trap};

pub fn build_linker(engine: &wasmtime::Engine) -> Result<Linker<HostState>, PluginError> {
    let mut linker = Linker::new(engine);

    // host_log(level: u32, ptr: u32, len: u32)
    linker
        .func_wrap(
            "env",
            "host_log",
            |mut caller: Caller<'_, HostState>,
             level: u32,
             ptr: u32,
             len: u32|
             -> wasmtime::Result<()> {
                let memory = caller
                    .get_export("memory")
                    .and_then(|m| m.into_memory())
                    .ok_or_else(|| wasmtime::Error::from(Trap::MemoryOutOfBounds))?;

                let (data, state) = memory.data_and_store_mut(&mut caller);
                let slice = data
                    .get(ptr as usize..(ptr + len) as usize)
                    .ok_or_else(|| wasmtime::Error::from(Trap::MemoryOutOfBounds))?;

                let msg = String::from_utf8_lossy(slice).to_string();
                let plugin_id = state.plugin_id.clone();
                tracing::info!(
                    target: "gitupload::plugins",
                    "[Plugin: {}][Level {}] {}",
                    plugin_id,
                    level,
                    msg
                );
                state.log_messages.push((level, msg));
                Ok(())
            },
        )
        .map_err(PluginError::WasmEngineError)?;

    // host_get_branch(out_ptr: u32, max_len: u32) -> u32
    linker
        .func_wrap(
            "env",
            "host_get_branch",
            |mut caller: Caller<'_, HostState>,
             out_ptr: u32,
             max_len: u32|
             -> wasmtime::Result<u32> {
                if !caller.data().capabilities.git_read {
                    tracing::warn!(
                        target: "gitupload::plugins",
                        "Plugin {} denied access to host_get_branch (missing git_read)",
                        caller.data().plugin_id
                    );
                    return Ok(0);
                }

                let branch = caller.data().current_branch.clone();
                let branch_bytes = branch.as_bytes();
                let copy_len = branch_bytes.len().min(max_len as usize);

                let memory = caller
                    .get_export("memory")
                    .and_then(|m| m.into_memory())
                    .ok_or_else(|| wasmtime::Error::from(Trap::MemoryOutOfBounds))?;

                let data = memory.data_mut(&mut caller);
                if (out_ptr as usize + copy_len) > data.len() {
                    return Err(wasmtime::Error::from(Trap::MemoryOutOfBounds));
                }

                data[out_ptr as usize..out_ptr as usize + copy_len]
                    .copy_from_slice(&branch_bytes[..copy_len]);
                Ok(copy_len as u32)
            },
        )
        .map_err(PluginError::WasmEngineError)?;

    // host_get_staged_files_count() -> u32
    linker
        .func_wrap(
            "env",
            "host_get_staged_files_count",
            |caller: Caller<'_, HostState>| -> wasmtime::Result<u32> {
                if !caller.data().capabilities.git_read {
                    return Ok(0);
                }
                Ok(caller.data().staged_files.len() as u32)
            },
        )
        .map_err(PluginError::WasmEngineError)?;

    // host_get_staged_file(index: u32, out_ptr: u32, max_len: u32) -> u32
    linker
        .func_wrap(
            "env",
            "host_get_staged_file",
            |mut caller: Caller<'_, HostState>,
             index: u32,
             out_ptr: u32,
             max_len: u32|
             -> wasmtime::Result<u32> {
                if !caller.data().capabilities.git_read {
                    return Ok(0);
                }

                let file_opt = caller.data().staged_files.get(index as usize).cloned();
                let Some(file_str) = file_opt else {
                    return Ok(0);
                };

                let file_bytes = file_str.as_bytes();
                let copy_len = file_bytes.len().min(max_len as usize);

                let memory = caller
                    .get_export("memory")
                    .and_then(|m| m.into_memory())
                    .ok_or_else(|| wasmtime::Error::from(Trap::MemoryOutOfBounds))?;

                let data = memory.data_mut(&mut caller);
                if (out_ptr as usize + copy_len) > data.len() {
                    return Err(wasmtime::Error::from(Trap::MemoryOutOfBounds));
                }

                data[out_ptr as usize..out_ptr as usize + copy_len]
                    .copy_from_slice(&file_bytes[..copy_len]);
                Ok(copy_len as u32)
            },
        )
        .map_err(PluginError::WasmEngineError)?;

    Ok(linker)
}
