//! OS-specific platform bridges: Windows atomic hot-swap & GPO Registry enforcement.

pub mod gpo_windows;
pub mod win32_swap;

pub use gpo_windows::{EffectiveEnterprisePolicy, EnterprisePolicyManager};
pub use win32_swap::{atomic_executable_swap, atomic_swap_at};
