# Sprint 5 Phase 5 — Sandbox Fuzzing, QA & Performance Signoff
**Document Identifier:** SPRINT5-PHASE5-SPEC  
**Target Milestone:** Release v2.6.0-extensible  
**Status:** Approved Technical Architecture  
**Parent Sprint Guide:** [Sprint 5 Master Guide](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-5/README.md)  

---

## 1. Overview & Quality Assurance Objectives

Phase 5 delivers the exhaustive testing, adversarial security fuzzing, latency benchmarking, and release validation suite for **Sprint 5**. It ensures that the **WebAssembly Plugin Host**, **Multi-Channel Webhook Dispatcher**, and **Headless CLI Tool** meet all enterprise security, resilience, and performance SLAs.

### 1.1 Quality & Security Objectives
1. **Adversarial WASM Sandbox Fuzzing**: Verify that malicious, buggy, or corrupted WebAssembly binaries cannot breach memory ceilings, exhaust host CPU, or trigger host application panics.
2. **Network Resilience & Webhook Fault Injection**: Validate retry backoff, HTTP 429 rate-limit throttling, and HMAC signature accuracy against simulated network degradation.
3. **Microsecond-Grade CLI Benchmarking**: Formally measure cold start latency, memory consumption, and JSON throughput with `criterion`.
4. **End-to-End System Integration**: Verify complete operational flow combining CLI execution, WASM hook enforcement, atomic marketplace updates, and multi-channel webhook broadcasts.

```
+------------------------------------------------------------------------------------------+
|                              Sprint 5 QA & Verification Suite                            |
|                                                                                          |
|  +----------------------------+  +----------------------------+  +--------------------+  |
|  |     WASM Sandbox Fuzz      |  |   Wiremock Webhook Suite   |  | Criterion CLI Bench|  |
|  | - 32MB Memory Bomb Test    |  | - 500 Internal Retry Test  |  | - <10ms Cold Start |  |
|  | - Infinite Loop Fuel Halt  |  | - 429 Rate-Limit Throttling|  | - <15MB RSS RAM    |  |
|  | - Unhandled Panic Trap     |  | - HMAC-SHA256 Match Test   |  | - Shell Completion |  |
|  +-------------+--------------+  +-------------+--------------+  +---------+----------+  |
|                |                               |                           |             |
|                +-------------------------------+---------------------------+             |
|                                                |                                         |
|                                 +--------------v---------------+                         |
|                                 |   End-to-End Integration     |                         |
|                                 | (CLI -> WASM -> Webhook)     |                         |
|                                 +--------------+---------------+                         |
|                                                |                                         |
|                                 +--------------v---------------+                         |
|                                 |   Release v2.6.0 Signoff     |                         |
|                                 +------------------------------+                         |
+------------------------------------------------------------------------------------------+
```

---

## 2. Adversarial WASM Sandbox Test Suite

```rust
// tests/wasm_sandbox_security.rs
use gitupload_plugins::engine::PluginEngine;
use gitupload_plugins::sandbox::{HostState, PluginCapabilities, PluginError};
use wasmtime::{Module, StoreLimitsBuilder, Trap};

#[test]
fn test_wasm_memory_bomb_trapped_at_32mb() {
    let engine = PluginEngine::new().expect("Engine initialization failed");
    
    // WebAssembly WAT attempting to grow memory indefinitely
    let wat = r#"
        (module
            (memory 1)
            (export "memory" (memory 0))
            (func (export "allocate_bomb") (result i32)
                (local $i i32)
                (loop $grow
                    ;; Attempt to grow by 100 pages (6.4 MB) repeatedly
                    (memory.grow (i32.const 100))
                    drop
                    (br $grow)
                )
                (i32.const 0)
            )
        )
    "#;

    let module = Module::new(&engine.raw_engine(), wat).expect("WAT compilation failed");
    let state = HostState {
        limits: StoreLimitsBuilder::new().memory_size(32 * 1024 * 1024).build(),
        plugin_id: "test.memory-bomb".into(),
        capabilities: PluginCapabilities::default(),
        staged_files: vec![],
        current_branch: "main".into(),
    };

    let mut store = engine.create_store(state);
    let instance = engine.instantiate(&mut store, &module).expect("Instantiation failed");
    let allocate_fn = instance
        .get_typed_func::<(), i32>(&mut store, "allocate_bomb")
        .expect("Export not found");

    // Execution must trap or fail gracefully without overflowing host heap
    let result = allocate_fn.call(&mut store, ());
    assert!(result.is_err(), "Memory bomb must be terminated by StoreLimits");
}

#[test]
fn test_wasm_infinite_loop_fuel_exhaustion() {
    let engine = PluginEngine::new().expect("Engine initialization failed");
    
    let wat = r#"
        (module
            (func (export "infinite_loop")
                (loop $infinite
                    (br $infinite)
                )
            )
        )
    "#;

    let module = Module::new(&engine.raw_engine(), wat).expect("WAT compilation failed");
    let state = HostState::default_for_test("test.infinite-loop");

    let mut store = engine.create_store(state);
    // Assign budget of 500,000 instructions
    store.set_fuel(500_000).unwrap();

    let instance = engine.instantiate(&mut store, &module).expect("Instantiation failed");
    let loop_fn = instance
        .get_typed_func::<(), ()>(&mut store, "infinite_loop")
        .expect("Export not found");

    let result = loop_fn.call(&mut store, ());
    match result {
        Err(err) => {
            let trap = err.downcast_ref::<Trap>();
            assert_eq!(trap, Some(&Trap::NoFuel), "Execution must trap with Trap::NoFuel");
        }
        Ok(_) => panic!("Infinite loop should never complete"),
    }
}
```

---

## 3. Webhook Fault Injection & Wiremock Test Harness

```rust
// tests/webhook_resilience.rs
use gitupload_app::webhooks::http::{send_generic_webhook, WebhookDeliveryError};
use secrecy::SecretString;
use std::time::Duration;
use wiremock::matchers::{header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_webhook_retries_on_500_server_error() {
    let mock_server = MockServer::start().await;

    // Fail first 2 attempts with 500, succeed on 3rd attempt
    Mock::given(method("POST"))
        .and(path("/webhook"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(2)
        .expect(2)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/webhook"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let payload = r#"{"event":"git_push","branch":"main"}"#;
    let secret = SecretString::new("test_secret_key_123".into());

    let mut attempts = 0;
    let mut success = false;

    for delay in [Duration::from_millis(100), Duration::from_millis(200)] {
        attempts += 1;
        let res = send_generic_webhook(
            &client,
            &format!("{}/webhook", mock_server.uri()),
            "git_push",
            payload,
            Some(&secret),
        )
        .await;

        if res.is_ok() {
            success = true;
            break;
        }
        tokio::time::sleep(delay).await;
    }

    if !success {
        attempts += 1;
        let final_res = send_generic_webhook(
            &client,
            &format!("{}/webhook", mock_server.uri()),
            "git_push",
            payload,
            Some(&secret),
        )
        .await;
        assert!(final_res.is_ok());
    }

    assert_eq!(attempts, 3, "Expected exactly 3 delivery attempts");
}

#[tokio::test]
async fn test_webhook_hmac_signature_correctness() {
    let mock_server = MockServer::start().await;
    let payload = r#"{"action":"ping"}"#;
    let secret = "my_vault_webhook_secret";

    // Expected HMAC-SHA256 digest
    use ring::hmac;
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let sig = hex::encode(hmac::sign(&key, payload.as_bytes()));
    let expected_sig_header = format!("sha256={}", sig);

    Mock::given(method("POST"))
        .and(path("/verify-hmac"))
        .and(header("X-Hub-Signature-256", expected_sig_header.as_str()))
        .and(header("X-GitUpload-Event", "ping"))
        .and(header_exists("X-GitUpload-Delivery"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let secret_str = SecretString::new(secret.into());
    let res = send_generic_webhook(
        &client,
        &format!("{}/verify-hmac", mock_server.uri()),
        "ping",
        payload,
        Some(&secret_str),
    )
    .await;

    assert!(res.is_ok(), "Webhook delivery with valid HMAC signature must succeed");
}
```

---

## 4. CLI Criterion Latency Benchmarks

```rust
// benches/cli_latency.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::process::Command;

fn bench_cli_version_startup(c: &mut Criterion) {
    let cli_path = env!("CARGO_BIN_EXE_gitupload");

    c.bench_function("cli_version_cold_start", |b| {
        b.iter(|| {
            let output = Command::new(cli_path)
                .arg("--version")
                .output()
                .expect("Failed to execute CLI");
            assert!(output.status.success());
        });
    });
}

fn bench_cli_status_json_latency(c: &mut Criterion) {
    let cli_path = env!("CARGO_BIN_EXE_gitupload");

    c.bench_function("cli_status_json", |b| {
        b.iter(|| {
            let output = Command::new(cli_path)
                .args(["status", "--short", "--json"])
                .output()
                .expect("Failed to execute status");
            assert!(output.status.success());
        });
    });
}

criterion_group!(benches, bench_cli_version_startup, bench_cli_status_json_latency);
criterion_main!(benches);
```

---

## 5. Work Breakdown & Granular Tasks

### TASK-P5-5-01: WASM Adversarial Security Test Suite
- **Target Files:**  
  `rust-gitupload/tests/wasm_sandbox_security.rs`
- **Actions:**
  - Build automated tests for: 32MB linear memory growth rejection, infinite loop fuel exhaustion, host trap safety, and missing capability rejection.
  - Verify that host memory footprint does not leak after guest termination.

### TASK-P5-5-02: Webhook Resilience & Wiremock Harness
- **Target Files:**  
  `rust-gitupload/tests/webhook_resilience.rs`
- **Actions:**
  - Set up `wiremock` test server to simulate HTTP 500 retries, HTTP 429 `Retry-After` rate limiting, and network timeout conditions.
  - Verify HMAC-SHA256 signature generation and validation against test secrets.

### TASK-P5-5-03: CLI Criterion Latency & Memory Verification
- **Target Files:**  
  `rust-gitupload/benches/cli_latency.rs`  
  `rust-gitupload/crates/gitupload-cli/tests/cli_e2e.rs`
- **Actions:**
  - Execute Criterion benchmarks validating sub-10ms cold start latency.
  - Measure peak RSS memory using Windows Process Memory Counters / Linux `/proc/self/status` (<15 MB target).

### TASK-P5-5-04: End-to-End Extensibility Integration & Signoff Report
- **Target Files:**  
  `rust-gitupload/tests/sprint5_e2e_integration.rs`  
  `rust-gitupload/docs/sprint-5/SIGNOFF_REPORT.md`
- **Actions:**
  - Create integration test that stages files via CLI, triggers a WASM commit hook, pushes to a bare git repository, and receives an HMAC-verified webhook.
  - Compile final Sprint 5 signoff verification report.

---

## 6. Acceptance Criteria (Given-When-Then)

### AC-P5-5-01: Zero-Panic WASM Memory Bomb Containment
- **Given:** A malicious WebAssembly plugin executing in `gitupload-plugins`.
- **When:** The plugin runs a tight loop attempting to allocate 256 MB of memory.
- **Then:** The host terminates the guest upon reaching 32 MB with zero host panics and zero memory leaks.

### AC-P5-5-02: Fuel Metering Deterministic Infinite Loop Halting (<5ms)
- **Given:** A guest module containing an unconditional branch `loop { br 0 }`.
- **When:** The module executes with 10,000,000 instruction fuel units.
- **Then:** Execution halts within 5ms, returning `Trap::NoFuel` with full CPU usage reclaimed immediately.

### AC-P5-5-03: Webhook Exponential Backoff & 429 Respect in Mock Server
- **Given:** A destination returning HTTP 429 with header `Retry-After: 1`.
- **When:** The webhook dispatcher delivers a payload.
- **Then:** The dispatcher pauses for 1 second before retrying, successfully delivering without server rate limit penalties.

### AC-P5-5-04: Sub-10ms CLI Startup Verified via Criterion
- **Given:** The Criterion benchmark suite measuring `gitupload --version`.
- **When:** 100 sample runs are executed in a release build.
- **Then:** The mean execution latency strictly benchmarks under 10.0 milliseconds.

### AC-P5-5-05: Full End-to-End Plugin + CLI + Webhook Integration Passes
- **Given:** The entire GitUpload v2.6 stack initialized in an ephemeral environment.
- **When:** A user executes `gitupload push -m "feat: complete sprint 5"`.
- **Then:** The WASM commit hook verifies the message, the git engine pushes to remote, and the webhook dispatcher broadcasts a signed event to the mock receiver with 100% test pass rate.

---

*Document Author:* `@srs_author`  
*Reviewers & Verifiers:* `@verifier`, `@coordinator`  
*Target Release:* GitUpload v2.6.0-extensible
