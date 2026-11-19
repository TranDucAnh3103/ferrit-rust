# Rule 02: Rust Coding Standards & Idiomatic Conventions
**Scope:** Rust Implementation, Type Safety, Code Quality, Clippy  
**Severity:** Critical  

---

## 1. Safety & Unsafe Code Policy

1. **`#![forbid(unsafe_code)]`**:
   - Every library crate MUST start with `#![forbid(unsafe_code)]` in its `src/lib.rs`.
   - The only authorized exception is Win32 FFI in `gitupload-app` (Single Instance Named Mutex) and `gitupload-git::cli` (Job Objects). These must be wrapped in strictly bounded `unsafe` blocks with detailed safety comments explaining why the call is sound.

2. **No Unhandled Panics in Production Paths**:
   - **Banned**: `.unwrap()`, `.expect()`, `panic!()` in non-test code.
   - Use `?` operator to bubble errors or handle `None` values with `.ok_or_else(...)`.
   - In unit tests only, `.unwrap()` and `.expect()` are permitted.

---

## 2. Error Handling Architecture

1. **Strongly Typed Domain Errors with `thiserror`**:
   - Each crate defines its own error enum derived using `#[derive(thiserror::Error, Debug)]`.
   - Example:
     ```rust
     #[derive(thiserror::Error, Debug)]
     pub enum GitError {
         #[error("Failed to open repository at '{0}': {1}")]
         RepoOpen(String, String),
         #[error("Merge conflict detected on {0} files")]
         Conflict(usize),
         #[error("Operation timed out after {0} seconds")]
         Timeout(u64),
     }
     ```

2. **Type Aliased Result**:
   - Every crate exposes a type alias: `pub type Result<T> = std::result::Result<T, DomainError>;`.

3. **IPC Error Mapping**:
   - All domain errors bubble up to `gitupload-app`, where they convert into `IpcErrorPayload` implementing `serde::Serialize` with `code`, `message`, and `recoverable` fields.

---

## 3. Idiomatic Rust Patterns

1. **Borrowing & Zero-Copy**:
   - Prefer `&Path` over `&PathBuf`.
   - Prefer `&str` over `&String`.
   - In diff and graph processing, use byte slices (`&[u8]`) or `Cow<'a, str>` to prevent needless heap allocations.

2. **Serde Serialization Tags**:
   - Enums serialized for frontend consumption MUST use standard internally or adjacently tagged serde representations:
     ```rust
     #[derive(Serialize, Deserialize)]
     #[serde(tag = "type", content = "payload")]
     pub enum JobEvent {
         Progress { percent: u32, message: String },
         Log { line: String },
         Completed { success: bool },
     }
     ```

3. **Struct Immutability by Default**:
   - Fields should be immutable unless explicit mutation is required.
   - Use the builder pattern for complex structs (e.g., `PushOptionsBuilder`, `CreatePrBuilder`).

---

## 4. Linting & Formatting Quality Gates

1. **Formatting**:
   - Code must be formatted with standard `rustfmt` rules (`cargo fmt --check`).
2. **Clippy Compliance**:
   - Must pass `cargo clippy --workspace --all-targets -- -D warnings`.
   - Pay special attention to:
     - `clippy::needless_borrow`
     - `clippy::redundant_clone`
     - `clippy::too_many_arguments` (use options structs instead)
     - `clippy::manual_let_else`
