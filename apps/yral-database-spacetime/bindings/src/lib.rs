// Thin wrapper that re-exports all generated bindings from the `generated/`
// subfolder. This file is hand-maintained — do not delete or regenerate.
//
// `spacetimedb-cli generate` writes all bindings (including `mod.rs`) into
// `src/generated/` (configured in `spacetime.json`). This wrapper avoids the
// CLI's `mod.rs` vs `lib.rs` naming conflict and its interactive deletion
// prompt, making `mise run spacetime-generate` fully non-interactive.

pub mod generated;

// Re-export all generated items at the crate root so consumers can use
// `yral_database_spacetime_bindings::DbConnection` etc. without the
// `generated::` prefix.
pub use generated::*;
