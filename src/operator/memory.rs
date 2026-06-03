//! In-memory backend builder for opendal operator (testing only).
//!
//! **WARNING:** The `Memory` operator is scoped to the current process.
//! Since git remote helpers are invoked as short-lived processes, data
//! stored in memory will be lost as soon as the command (push/fetch)
//! completes. This backend is only useful for unit tests.
//!
//! `Memory` operator is useful in unit tests and local debugging.

use anyhow::Result;
use opendal::Operator;

/// Build a Memory `Operator`.
pub fn build_memory() -> Result<Operator> {
    use opendal::services::Memory;
    Ok(Operator::new(Memory::default())?.finish())
}
