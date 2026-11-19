#![forbid(unsafe_code)]

use gitupload_core::plugin_manifest::PluginCapabilities;
use gitupload_plugins::engine::PluginEngine;
use gitupload_plugins::linker::build_linker;
use gitupload_plugins::sandbox::HostState;
use wasmtime::{Module, StoreLimitsBuilder};

#[test]
fn test_wasm_memory_bomb_trapped_at_32mb() {
    let engine = PluginEngine::new().expect("Engine initialization failed");

    // WebAssembly WAT attempting to grow memory indefinitely; returns -1 when StoreLimits denies allocation
    let wat = r#"
        (module
            (memory 1)
            (export "memory" (memory 0))
            (func (export "allocate_bomb") (result i32)
                (loop $grow
                    (if (i32.eq (memory.grow (i32.const 100)) (i32.const -1))
                        (then (return (i32.const -1)))
                    )
                    (br $grow)
                )
                (i32.const 0)
            )
        )
    "#;

    let module = Module::new(engine.raw_engine(), wat).expect("WAT compilation failed");
    let state = HostState {
        limits: StoreLimitsBuilder::new()
            .memory_size(32 * 1024 * 1024)
            .build(),
        plugin_id: "test.memory-bomb".into(),
        capabilities: PluginCapabilities {
            max_memory_mb: 32,
            ..Default::default()
        },
        staged_files: vec![],
        current_branch: "main".into(),
        log_messages: Vec::new(),
    };

    let mut store = engine.create_store(state);
    let instance = engine
        .instantiate(&mut store, &module)
        .expect("Instantiation failed");
    let allocate_fn = instance
        .get_typed_func::<(), i32>(&mut store, "allocate_bomb")
        .expect("Export allocate_bomb not found");

    let result = allocate_fn.call(&mut store, ());
    assert!(
        result.is_ok(),
        "Host execution must remain stable and not panic"
    );
    assert_eq!(
        result.unwrap(),
        -1,
        "Memory growth beyond 32MB ceiling must be rejected with -1"
    );
}

#[test]
fn test_wasm_fuel_metering_deterministic_consumption() {
    let engine = PluginEngine::new().expect("Engine initialization failed");

    let wat = r#"
        (module
            (func (export "compute_heavy") (result i32)
                (local $i i32)
                (local.set $i (i32.const 10000))
                (loop $loop
                    (local.set $i (i32.sub (local.get $i) (i32.const 1)))
                    (br_if $loop (i32.gt_s (local.get $i) (i32.const 0)))
                )
                (local.get $i)
            )
        )
    "#;

    let module = Module::new(engine.raw_engine(), wat).expect("WAT compilation failed");
    let state = HostState::default_for_test("test.fuel-consumption");

    let mut store = engine.create_store(state);
    store.set_fuel(10_000_000).expect("Failed to set fuel");
    let initial_fuel = store.get_fuel().expect("Failed to get fuel");

    let instance = engine
        .instantiate(&mut store, &module)
        .expect("Instantiation failed");
    let compute_fn = instance
        .get_typed_func::<(), i32>(&mut store, "compute_heavy")
        .expect("Export compute_heavy not found");

    let result = compute_fn.call(&mut store, ());
    assert_eq!(result.unwrap(), 0);
    let remaining_fuel = store.get_fuel().expect("Failed to get fuel");
    assert!(
        remaining_fuel < initial_fuel,
        "Execution must consume fuel deterministically: initial {}, remaining {}",
        initial_fuel,
        remaining_fuel
    );
}

#[test]
fn test_wasm_host_logger_and_state_isolation() {
    let engine = PluginEngine::new().expect("Engine initialization failed");
    let linker = build_linker(engine.raw_engine()).expect("Linker build failed");

    let wat = r#"
        (module
            (import "env" "host_log" (func $host_log (param i32 i32 i32)))
            (memory (export "memory") 1)
            (data (i32.const 0) "Hello from sandboxed WASM plugin")
            (func (export "log_test")
                (call $host_log (i32.const 1) (i32.const 0) (i32.const 32))
            )
        )
    "#;

    let module = Module::new(engine.raw_engine(), wat).expect("WAT compilation failed");
    let state = HostState::default_for_test("test.logger");
    let mut store = engine.create_store(state);

    let instance = linker
        .instantiate(&mut store, &module)
        .expect("Instantiation failed");

    let log_fn = instance
        .get_typed_func::<(), ()>(&mut store, "log_test")
        .expect("Export log_test not found");

    log_fn.call(&mut store, ()).expect("Host log call failed");

    assert_eq!(store.data().log_messages.len(), 1);
    assert_eq!(store.data().log_messages[0].0, 1);
    assert_eq!(
        store.data().log_messages[0].1,
        "Hello from sandboxed WASM plugin"
    );
}

#[test]
fn test_wasm_capability_denial_without_permission() {
    let engine = PluginEngine::new().expect("Engine initialization failed");
    let linker = build_linker(engine.raw_engine()).expect("Linker build failed");

    let wat = r#"
        (module
            (import "env" "host_get_branch" (func $host_get_branch (param i32 i32) (result i32)))
            (memory (export "memory") 1)
            (func (export "call_branch") (result i32)
                (call $host_get_branch (i32.const 0) (i32.const 64))
            )
        )
    "#;

    let module = Module::new(engine.raw_engine(), wat).expect("WAT compilation failed");

    // State with git_read = false
    let caps = PluginCapabilities {
        git_read: false,
        ..Default::default()
    };

    let state = HostState {
        limits: StoreLimitsBuilder::new()
            .memory_size(32 * 1024 * 1024)
            .build(),
        plugin_id: "test.cap-denial".into(),
        capabilities: caps,
        staged_files: vec!["file.txt".to_string()],
        current_branch: "feat/unauthorized".into(),
        log_messages: Vec::new(),
    };

    let mut store = engine.create_store(state);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("Instantiation failed");

    let call_fn = instance
        .get_typed_func::<(), i32>(&mut store, "call_branch")
        .expect("Export call_branch not found");

    let bytes_copied = call_fn.call(&mut store, ()).expect("Host call failed");
    assert_eq!(
        bytes_copied, 0,
        "Calling host_get_branch without git_read must return 0 bytes"
    );
}
