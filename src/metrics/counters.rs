// Counters module — re-exports are available from `state.rs`.
//
// This module exists purely for module-boundary clarity per
// architecture_doc.md §5.4. The actual struct lives in `state.rs` because
// `AppState` owns it and both are constructed together.
