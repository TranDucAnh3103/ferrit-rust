use crate::engine::PluginEngine;
use crate::linker::build_linker;
use crate::sandbox::{HostState, PluginError};
use gitupload_core::plugin_manifest::PluginManifest;
use std::path::Path;
use std::sync::Arc;
use wasmtime::{Linker, Module};

pub struct PluginRunner {
    engine: Arc<PluginEngine>,
    linker: Linker<HostState>,
}

impl PluginRunner {
    pub fn new() -> Result<Self, PluginError> {
        let engine = Arc::new(PluginEngine::new()?);
        let linker = build_linker(engine.raw_engine())?;
        Ok(Self { engine, linker })
    }

    pub fn engine(&self) -> &PluginEngine {
        &self.engine
    }

    /// Executes `plugin_on_commit_msg` entrypoint on a target WASM plugin.
    /// Returns `Ok(())` if accepted (return code 0), or `Err(PluginError)` if rejected or trapped.
    pub fn run_commit_msg_hook(
        &self,
        plugin_dir: &Path,
        manifest: &PluginManifest,
        commit_msg: &str,
        branch: &str,
        staged_files: &[String],
    ) -> Result<(), PluginError> {
        let wasm_path = plugin_dir.join(&manifest.plugin.entrypoint);
        let wasm_bytes = std::fs::read(&wasm_path).map_err(PluginError::Io)?;
        let module = Module::new(self.engine.raw_engine(), &wasm_bytes)
            .map_err(PluginError::WasmEngineError)?;

        let state = HostState {
            limits: wasmtime::StoreLimitsBuilder::new()
                .memory_size((manifest.capabilities.max_memory_mb.min(32) as usize) * 1024 * 1024)
                .instances(1)
                .tables(10)
                .build(),
            plugin_id: manifest.plugin.id.clone(),
            capabilities: manifest.capabilities.clone(),
            staged_files: staged_files.to_vec(),
            current_branch: branch.to_string(),
            log_messages: Vec::new(),
        };

        let mut store = self.engine.create_store(state);
        let instance = self
            .linker
            .instantiate(&mut store, &module)
            .map_err(PluginError::WasmEngineError)?;

        // Optional plugin_init
        if let Ok(init_fn) = instance.get_typed_func::<(), i32>(&mut store, "plugin_init") {
            let _ = init_fn.call(&mut store, ());
        }

        // plugin_on_commit_msg(ptr: u32, len: u32) -> i32
        if let Ok(hook_fn) =
            instance.get_typed_func::<(u32, u32), i32>(&mut store, "plugin_on_commit_msg")
        {
            let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
                PluginError::ExecutionTrapped("Plugin missing 'memory' export".into())
            })?;

            let msg_bytes = commit_msg.as_bytes();
            let msg_len = msg_bytes.len() as u32;

            // Write msg_bytes into guest linear memory at offset 1024 (safe offset past initial data)
            let offset: u32 = 1024;
            {
                let data = memory.data_mut(&mut store);
                if (offset as usize + msg_bytes.len()) > data.len() {
                    return Err(PluginError::MemoryCeilingExceeded);
                }
                data[offset as usize..offset as usize + msg_bytes.len()].copy_from_slice(msg_bytes);
            }

            let result_code = hook_fn.call(&mut store, (offset, msg_len)).map_err(|e| {
                if let Some(trap) = e.downcast_ref::<wasmtime::Trap>() {
                    if *trap == wasmtime::Trap::OutOfFuel {
                        return PluginError::FuelExhausted;
                    }
                }
                PluginError::ExecutionTrapped(e.to_string())
            })?;

            if result_code != 0 {
                return Err(PluginError::ExecutionTrapped(format!(
                    "Commit hook rejected commit message (code {})",
                    result_code
                )));
            }
        }

        Ok(())
    }
}
