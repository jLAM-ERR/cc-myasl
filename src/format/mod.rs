//! Format engine for the status-line template language.
//!
//! `parser` tokenises a template string; `values` provides the
//! rendering helpers (bars, clocks, countdowns).  The full `render`
//! API and the placeholder catalogue land in Task 4.

pub mod parser;
pub mod values;
