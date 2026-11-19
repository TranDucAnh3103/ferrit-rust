#![forbid(unsafe_code)]
//! Interactive Git Graph (DAG) revwalk builder, swimlane router, and contextual commit actions.

pub mod actions;
pub mod engine;
pub mod swimlane;

pub use actions::GitGraphActions;
pub use engine::{DagRevwalkEngine, GitGraphEngine};
pub use swimlane::SwimlaneRouter;
