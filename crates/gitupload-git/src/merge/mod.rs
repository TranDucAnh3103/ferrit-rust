//! Interactive 3-way merge conflict calculation, hunk parsing, resolution, and atomic merge commit construction.

pub mod calculator;
pub mod commit_builder;
pub mod parser;
pub mod resolver;

pub use calculator::MergeCalculator;
pub use commit_builder::CommitBuilder;
pub use parser::ConflictParser;
pub use resolver::ConflictResolver;
