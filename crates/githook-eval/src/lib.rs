// Redesigned Execution System
pub mod value;
pub mod executor;

// Keep stdlib and package resolver
mod stdlib;
pub mod package_resolver;

// Re-exports for convenience
pub use value::Value;
pub use executor::{Executor, ExecutionResult};