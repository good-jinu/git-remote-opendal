//! In-memory backend builder for opendal operator (testing only).
//!
//! `Memory` operator is useful in unit tests and local debugging.

use anyhow::Result;
use opendal::Operator;

/// Build a Memory `Operator`.
pub fn build_memory() -> Result<Operator> {
    use opendal::services::Memory;
    Ok(Operator::new(Memory::default())?.finish())
}
