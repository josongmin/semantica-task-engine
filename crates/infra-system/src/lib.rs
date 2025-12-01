// Semantica Infrastructure - System Adapters
// Implements: SystemProbe, TaskExecutor (ADR-002)

pub mod subprocess_executor;
pub mod system_probe_impl;

pub use subprocess_executor::SubprocessExecutor;
pub use system_probe_impl::SystemProbeImpl;
