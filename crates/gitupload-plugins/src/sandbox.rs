use gitupload_core::plugin_manifest::PluginCapabilities;
use thiserror::Error;
use wasmtime::{ResourceLimiter, StoreLimits};

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin manifest error: {0}")]
    ManifestError(String),
    #[error("WASM engine error: {0}")]
    WasmEngineError(#[from] wasmtime::Error),
    #[error("Plugin execution trapped: {0}")]
    ExecutionTrapped(String),
    #[error("Permission denied: capability '{0}' not granted")]
    PermissionDenied(String),
    #[error("Linear memory exceeded 32MB ceiling")]
    MemoryCeilingExceeded,
    #[error("Fuel exhausted: execution exceeded instruction budget")]
    FuelExhausted,
    #[error("Verification error: {0}")]
    Verification(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Archive error: {0}")]
    Archive(String),
}

pub struct HostState {
    pub limits: StoreLimits,
    pub plugin_id: String,
    pub capabilities: PluginCapabilities,
    pub staged_files: Vec<String>,
    pub current_branch: String,
    pub log_messages: Vec<(u32, String)>,
}

impl HostState {
    pub fn new(
        plugin_id: String,
        capabilities: PluginCapabilities,
        current_branch: String,
        staged_files: Vec<String>,
        limits: StoreLimits,
    ) -> Self {
        Self {
            limits,
            plugin_id,
            capabilities,
            staged_files,
            current_branch,
            log_messages: Vec::new(),
        }
    }

    pub fn default_for_test(plugin_id: &str) -> Self {
        use wasmtime::StoreLimitsBuilder;
        Self {
            limits: StoreLimitsBuilder::new()
                .memory_size(32 * 1024 * 1024)
                .build(),
            plugin_id: plugin_id.to_string(),
            capabilities: PluginCapabilities::default(),
            staged_files: Vec::new(),
            current_branch: "main".to_string(),
            log_messages: Vec::new(),
        }
    }
}

impl ResourceLimiter for HostState {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        self.limits.memory_growing(current, desired, maximum)
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        self.limits.table_growing(current, desired, maximum)
    }
}
