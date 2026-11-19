#![forbid(unsafe_code)]
//! Deterministic parser extracting structured [`ConflictHunk`] records from 2-way and `diff3` conflict markers.

use gitupload_core::models::ConflictHunk;

/// Stateless line-scanner extracting structured conflict hunks without mutating UTF-8 codepoints.
pub struct ConflictParser;

impl ConflictParser {
    /// Parses standard (`<<<<<<<`/`=======`/`>>>>>>>`) and `diff3` (`|||||||`) conflict markers in `text`.
    pub fn parse(file_path: &str, text: &str) -> Vec<ConflictHunk> {
        let normalized = text.replace("\r\n", "\n");
        let lines: Vec<&str> = normalized.split('\n').collect();

        #[derive(Copy, Clone, PartialEq, Eq)]
        enum Section {
            None,
            Ours,
            Base,
            Theirs,
        }

        let mut hunks = Vec::new();
        let mut section = Section::None;
        let mut start_line_ours = 0usize;
        let mut end_line_ours = 0usize;
        let mut start_line_theirs = 0usize;
        let mut ours_lines: Vec<&str> = Vec::new();
        let mut base_lines: Option<Vec<&str>> = None;
        let mut theirs_lines: Vec<&str> = Vec::new();

        for (idx, line) in lines.iter().enumerate() {
            let line_no = idx + 1;
            if line.starts_with("<<<<<<<") {
                section = Section::Ours;
                start_line_ours = line_no;
                end_line_ours = line_no;
                start_line_theirs = line_no;
                ours_lines.clear();
                base_lines = None;
                theirs_lines.clear();
                continue;
            }

            if section != Section::None && line.starts_with("|||||||") {
                section = Section::Base;
                base_lines = Some(Vec::new());
                continue;
            }

            if section != Section::None && line.starts_with("=======") {
                end_line_ours = line_no.saturating_sub(1);
                start_line_theirs = line_no + 1;
                section = Section::Theirs;
                continue;
            }

            if section != Section::None && line.starts_with(">>>>>>>") {
                let hunk_index = hunks.len();
                let hunk_id = format!("{}:{}", fnv1a_prefix(file_path), hunk_index);
                hunks.push(ConflictHunk {
                    hunk_id,
                    file_path: file_path.to_string(),
                    hunk_index,
                    start_line_ours,
                    end_line_ours,
                    start_line_theirs,
                    end_line_theirs: line_no,
                    content_ours: ours_lines.join("\n"),
                    content_theirs: theirs_lines.join("\n"),
                    content_base: base_lines.take().map(|v| v.join("\n")),
                });
                section = Section::None;
                continue;
            }

            match section {
                Section::Ours => ours_lines.push(line),
                Section::Base => {
                    if let Some(ref mut base) = base_lines {
                        base.push(line);
                    }
                }
                Section::Theirs => theirs_lines.push(line),
                Section::None => {}
            }
        }

        hunks
    }
}

fn fnv1a_prefix(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:08x}")[..8].to_string()
}
