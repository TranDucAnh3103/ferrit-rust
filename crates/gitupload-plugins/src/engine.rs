use crate::sandbox::{HostState, PluginError};
use wasmtime::{Config, Engine, Instance, Linker, Module, Store, StoreLimitsBuilder};

pub struct PluginEngine {
    engine: Engine,
}

impl PluginEngine {
    pub fn new() -> Result<Self, PluginError> {
        let mut config = Config::new();
        // Enable instruction fuel metering to terminate infinite loops
        config.consume_fuel(true);
        // Explicit software bounds checking rather than hardware signals for Windows CET stability
        config.signals_based_traps(false);
        // Strict 32-bit linear memory sandboxing
        config.wasm_memory64(false);

        let engine = Engine::new(&config).map_err(PluginError::WasmEngineError)?;
        Ok(Self { engine })
    }

    pub fn raw_engine(&self) -> &Engine {
        &self.engine
    }

    pub fn create_store(&self, state: HostState) -> Store<HostState> {
        let max_mb = if state.capabilities.max_memory_mb == 0 {
            16
        } else {
            state.capabilities.max_memory_mb.min(32)
        };
        let max_bytes = (max_mb as usize) * 1024 * 1024;
        let limits = StoreLimitsBuilder::new()
            .memory_size(max_bytes)
            .instances(1)
            .tables(10)
            .build();

        let mut store_state = state;
        store_state.limits = limits;

        let mut store = Store::new(&self.engine, store_state);
        store.limiter(|s| s);
        // Assign 10,000,000 fuel units per execution
        let _ = store.set_fuel(10_000_000);
        store
    }

    pub fn instantiate(
        &self,
        store: &mut Store<HostState>,
        module: &Module,
    ) -> Result<Instance, PluginError> {
        let linker = Linker::new(&self.engine);
        linker
            .instantiate(store, module)
            .map_err(PluginError::WasmEngineError)
    }

    pub fn instantiate_with_linker(
        &self,
        store: &mut Store<HostState>,
        module: &Module,
        linker: &Linker<HostState>,
    ) -> Result<Instance, PluginError> {
        linker
            .instantiate(store, module)
            .map_err(PluginError::WasmEngineError)
    }
}
