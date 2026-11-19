//! Topological swimlane column allocator and Bezier edge generator for DAG rendering.

use gitupload_core::models::{BranchSwimlane, GraphLineSegment, GraphLineType};
use std::collections::HashMap;

const SWIMLANE_PALETTE: &[&str] = &[
    "#1f6feb", // Primer Blue
    "#2ea043", // Primer Green
    "#8957e5", // Primer Purple
    "#d29922", // Primer Amber
    "#bf3989", // Primer Pink
    "#39c5cf", // Primer Cyan
];

/// Dynamically allocates branch swimlane columns and computes parent-child line segments.
pub struct SwimlaneRouter;

impl SwimlaneRouter {
    /// Returns the Primer hex color assigned to column `col`.
    pub fn color_for_column(col: usize) -> String {
        SWIMLANE_PALETTE[col % SWIMLANE_PALETTE.len()].to_string()
    }

    /// Assigns column coordinates and outgoing parent edges to a topologically sorted slice of
    /// `(oid, parent_oids)` tuples.
    pub fn route(
        commits: &[(String, Vec<String>)],
    ) -> (Vec<(usize, Vec<GraphLineSegment>)>, Vec<BranchSwimlane>) {
        let mut oid_to_row: HashMap<&str, usize> = HashMap::with_capacity(commits.len());
        for (row, (oid, _)) in commits.iter().enumerate() {
            oid_to_row.insert(oid.as_str(), row);
        }

        // Active lanes each track the next expected parent OID along that vertical column.
        let mut active_lanes: Vec<Option<String>> = Vec::new();
        let mut result = Vec::with_capacity(commits.len());
        let mut max_col_seen = 0usize;

        for (row, (oid, parents)) in commits.iter().enumerate() {
            // 1. Find primary lane expecting this commit OID, or allocate the lowest free lane.
            let col = if let Some(idx) = active_lanes
                .iter()
                .position(|slot| slot.as_deref() == Some(oid.as_str()))
            {
                idx
            } else if let Some(free_idx) = active_lanes.iter().position(Option::is_none) {
                active_lanes[free_idx] = Some(oid.clone());
                free_idx
            } else {
                active_lanes.push(Some(oid.clone()));
                active_lanes.len() - 1
            };

            max_col_seen = max_col_seen.max(col);

            // Clear any duplicate lanes that were also waiting on `oid` (converging branches).
            for (other_idx, slot) in active_lanes.iter_mut().enumerate() {
                if other_idx != col && slot.as_deref() == Some(oid.as_str()) {
                    *slot = None;
                }
            }

            let mut edges = Vec::with_capacity(parents.len());

            if let Some(first_parent) = parents.first() {
                // Check if another column already expects `first_parent`.
                let existing_parent_col = active_lanes
                    .iter()
                    .enumerate()
                    .find(|(idx, slot)| {
                        *idx != col && slot.as_deref() == Some(first_parent.as_str())
                    })
                    .map(|(idx, _)| idx);

                let to_col = if let Some(existing_col) = existing_parent_col {
                    // Merge current lane back into existing parent lane and free `col`.
                    active_lanes[col] = None;
                    existing_col
                } else {
                    active_lanes[col] = Some(first_parent.clone());
                    col
                };

                let to_row = oid_to_row
                    .get(first_parent.as_str())
                    .copied()
                    .unwrap_or(row + 1);
                let line_type = if to_col == col {
                    GraphLineType::Straight
                } else {
                    GraphLineType::MergeIn
                };
                edges.push(GraphLineSegment {
                    from_col: col,
                    to_col,
                    from_row: row,
                    to_row,
                    line_type,
                    color: Self::color_for_column(col),
                });
            } else {
                // Root commit ends this lane.
                active_lanes[col] = None;
            }

            // Secondary parents (merge commits) allocate or reuse a branch lane.
            for secondary_parent in parents.iter().skip(1) {
                let target_col = if let Some(existing_col) = active_lanes
                    .iter()
                    .position(|slot| slot.as_deref() == Some(secondary_parent.as_str()))
                {
                    existing_col
                } else if let Some(free_idx) = active_lanes.iter().position(Option::is_none) {
                    active_lanes[free_idx] = Some(secondary_parent.clone());
                    free_idx
                } else {
                    active_lanes.push(Some(secondary_parent.clone()));
                    active_lanes.len() - 1
                };
                max_col_seen = max_col_seen.max(target_col);

                let to_row = oid_to_row
                    .get(secondary_parent.as_str())
                    .copied()
                    .unwrap_or(row + 1);
                edges.push(GraphLineSegment {
                    from_col: col,
                    to_col: target_col,
                    from_row: row,
                    to_row,
                    line_type: if target_col == col {
                        GraphLineType::Straight
                    } else {
                        GraphLineType::BranchOut
                    },
                    color: Self::color_for_column(target_col),
                });
            }

            result.push((col, edges));
        }

        let swimlanes = (0..=max_col_seen)
            .map(|idx| BranchSwimlane {
                index: idx,
                branch_name: None,
                color: Self::color_for_column(idx),
                is_active: active_lanes.get(idx).is_some_and(Option::is_some),
            })
            .collect();

        (result, swimlanes)
    }
}
