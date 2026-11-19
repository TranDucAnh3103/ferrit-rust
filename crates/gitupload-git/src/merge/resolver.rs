#![forbid(unsafe_code)]
//! Deterministic 1-click hunk applicator reconstructing clean resolved source buffers.

use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::{ConflictHunk, ConflictResolutionChoice, FileResolutionRequest};
use std::collections::HashMap;

/// Applies per-hunk or full-buffer conflict resolution choices onto conflicted source text.
pub struct ConflictResolver;

impl ConflictResolver {
    /// Applies every [`FileResolutionRequest`] decision onto `raw_conflicted_text` and returns clean merged source.
    pub fn apply_resolutions(
        raw_conflicted_text: &str,
        hunks: &[ConflictHunk],
        request: &FileResolutionRequest,
    ) -> GitUploadResult<String> {
        if let Some(custom_full) = &request.custom_content {
            return Ok(custom_full.clone());
        }

        let normalized = raw_conflicted_text.replace("\r\n", "\n");
        let lines: Vec<&str> = normalized.split('\n').collect();

        let choice_map: HashMap<usize, &ConflictResolutionChoice> = request
            .chunk_resolutions
            .iter()
            .map(|(idx, choice)| (*idx, choice))
            .collect();

        for hunk in hunks {
            if !choice_map.contains_key(&hunk.hunk_index) {
                return Err(GitUploadError::Git(format!(
                    "Unresolved conflict hunk #{} in '{}'",
                    hunk.hunk_index, request.file_path
                )));
            }
        }

        let mut output_lines: Vec<String> = Vec::with_capacity(lines.len());
        let mut cursor_line = 1usize; // 1-based line number matching ConflictHunk bounds

        for hunk in hunks {
            while cursor_line < hunk.start_line_ours && cursor_line <= lines.len() {
                output_lines.push(lines[cursor_line - 1].to_string());
                cursor_line += 1;
            }

            let choice = choice_map.get(&hunk.hunk_index).ok_or_else(|| {
                GitUploadError::Git(format!("Missing hunk resolution #{}", hunk.hunk_index))
            })?;

            let replacement = match choice {
                ConflictResolutionChoice::AcceptCurrent => hunk.content_ours.clone(),
                ConflictResolutionChoice::AcceptIncoming => hunk.content_theirs.clone(),
                ConflictResolutionChoice::AcceptBoth => {
                    if hunk.content_ours.is_empty() {
                        hunk.content_theirs.clone()
                    } else if hunk.content_theirs.is_empty() {
                        hunk.content_ours.clone()
                    } else {
                        format!("{}\n{}", hunk.content_ours, hunk.content_theirs)
                    }
                }
                ConflictResolutionChoice::Custom(text) => text.clone(),
            };

            if !replacement.is_empty() {
                for seg in replacement.replace("\r\n", "\n").split('\n') {
                    output_lines.push(seg.to_string());
                }
            }

            cursor_line = hunk.end_line_theirs + 1;
        }

        while cursor_line <= lines.len() {
            output_lines.push(lines[cursor_line - 1].to_string());
            cursor_line += 1;
        }

        Ok(output_lines.join("\n"))
    }
}
