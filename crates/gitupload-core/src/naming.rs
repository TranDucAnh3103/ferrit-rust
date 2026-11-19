//! Repository naming, slugification, validation, and legacy homework filename parsing.

use crate::errors::{GitUploadError, GitUploadResult};
use crate::models::{AppSettings, PushMode, PushPayload};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

static RE_SLUG_INVALID: OnceLock<Option<Regex>> = OnceLock::new();
static RE_DASHES: OnceLock<Option<Regex>> = OnceLock::new();
static RE_REPO_VALID: OnceLock<Option<Regex>> = OnceLock::new();
static RE_MINI_INVALID: OnceLock<Option<Regex>> = OnceLock::new();
static RE_FILE_HOMEWORK: OnceLock<Option<Regex>> = OnceLock::new();

/// Extracted assignment metadata from structured filenames like `SS15_HW02_DucAnh_HE170001.java`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHomeworkMetadata {
    /// Session number parsed from `SS<N>`.
    pub session: u32,
    /// Exercise number parsed from `HW<N>` or `EX<N>`.
    pub exercise: u32,
    /// Optional student name segment.
    pub student_name: Option<String>,
    /// Optional student ID segment.
    pub student_id: Option<String>,
}

fn re_slug_invalid() -> Option<&'static Regex> {
    RE_SLUG_INVALID
        .get_or_init(|| Regex::new(r"[^a-zA-Z0-9._-]").ok())
        .as_ref()
}

fn re_dashes() -> Option<&'static Regex> {
    RE_DASHES.get_or_init(|| Regex::new(r"-+").ok()).as_ref()
}

fn re_repo_valid() -> Option<&'static Regex> {
    RE_REPO_VALID
        .get_or_init(|| Regex::new(r"^[a-zA-Z0-9._-]+$").ok())
        .as_ref()
}

fn re_mini_invalid() -> Option<&'static Regex> {
    RE_MINI_INVALID
        .get_or_init(|| Regex::new(r"[^a-z0-9._-]+").ok())
        .as_ref()
}

fn re_file_homework() -> Option<&'static Regex> {
    RE_FILE_HOMEWORK
        .get_or_init(|| {
            Regex::new(
                r"(?i)SS(\d+)[_-](?:HW|EX)(\d+)(?:[_-]([a-zA-Z0-9]+))?(?:[_-]([a-zA-Z0-9]+))?",
            )
            .ok()
        })
        .as_ref()
}

/// Converts arbitrary text into a GitHub-compliant repository slug while preserving case.
pub fn slugify_repo_name(name: &str) -> String {
    let trimmed = name.trim().replace(' ', "-");
    let cleaned = match re_slug_invalid() {
        Some(re) => re.replace_all(&trimmed, "").into_owned(),
        None => trimmed
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '_' || *c == '-')
            .collect(),
    };
    let normalized = match re_dashes() {
        Some(re) => re.replace_all(&cleaned, "-").into_owned(),
        None => cleaned,
    };
    normalized
        .trim_matches(|c| c == '.' || c == '-')
        .to_string()
}

/// Normalizes mini-project names in lowercase with GitHub-safe characters.
pub fn sanitize_mini_project_text(text: &str) -> GitUploadResult<String> {
    let lowered = text.trim().to_lowercase();
    if lowered.is_empty() {
        return Err(GitUploadError::Config(
            "Tên miniproject không được để trống".into(),
        ));
    }
    let replaced = match re_mini_invalid() {
        Some(re) => re.replace_all(&lowered, "-").into_owned(),
        None => lowered
            .chars()
            .map(|c| {
                if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-'
                {
                    c
                } else {
                    '-'
                }
            })
            .collect(),
    };
    let collapsed = match re_dashes() {
        Some(re) => re.replace_all(&replaced, "-").into_owned(),
        None => replaced,
    };
    let normalized = collapsed.trim_matches(|c| c == '.' || c == '-').to_string();
    if normalized.is_empty() {
        return Err(GitUploadError::Config(
            "Tên miniproject không hợp lệ sau khi chuẩn hóa".into(),
        ));
    }
    Ok(normalized)
}

/// Validates a repository name against GitHub naming rules and traversal safeguards.
pub fn validate_repository_name(name: &str) -> GitUploadResult<()> {
    if name.is_empty() {
        return Err(GitUploadError::Config(
            "Tên repository không được để trống".into(),
        ));
    }
    if name.len() > 100 {
        return Err(GitUploadError::Config(
            "Tên repository chỉ được tối đa 100 ký tự".into(),
        ));
    }
    let is_valid_chars = match re_repo_valid() {
        Some(re) => re.is_match(name),
        None => name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-'),
    };
    if !is_valid_chars {
        return Err(GitUploadError::Config(
            "Tên repository chỉ được gồm chữ, số, '.', '_' hoặc '-'".into(),
        ));
    }
    if name.starts_with('.') || name.ends_with('.') || name.starts_with('-') || name.ends_with('-')
    {
        return Err(GitUploadError::Config(
            "Tên repository không được bắt đầu hoặc kết thúc bằng '.' hoặc '-'".into(),
        ));
    }
    if name.contains("..") {
        return Err(GitUploadError::Config(
            "Tên repository không được chứa '..'".into(),
        ));
    }
    Ok(())
}

/// Parses homework metadata from filenames like `SS15_HW02_DucAnh_HE170001.java`.
pub fn parse_file_metadata(filename: &str) -> Option<FileHomeworkMetadata> {
    let base_name = Path::new(filename)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);
    let stem = Path::new(base_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(base_name);

    let re = re_file_homework()?;
    let caps = re.captures(stem)?;
    let session = caps.get(1)?.as_str().parse().ok()?;
    let exercise = caps.get(2)?.as_str().parse().ok()?;
    let student_name = caps.get(3).map(|m| m.as_str().to_string());
    let student_id = caps.get(4).map(|m| m.as_str().to_string());

    Some(FileHomeworkMetadata {
        session,
        exercise,
        student_name,
        student_id,
    })
}

/// Formats a repository name template supporting `%02d`, `%d`, `%s`, `{session}`, `{exercise}`,
/// `{session:02}`, and `{exercise:02}` placeholders.
pub fn format_repo_name(pattern: &str, session: u32, exercise: u32) -> String {
    let tmpl = if pattern.trim().is_empty() {
        "session%02d-ex%02d-homework"
    } else {
        pattern
    };

    if tmpl.contains("{session") || tmpl.contains("{exercise") {
        return tmpl
            .replace("{session:02}", &format!("{session:02}"))
            .replace("{exercise:02}", &format!("{exercise:02}"))
            .replace("{session}", &session.to_string())
            .replace("{exercise}", &exercise.to_string());
    }

    let mut out = String::new();
    let mut chars = tmpl.chars().peekable();
    let values = [session, exercise];
    let mut arg_idx = 0usize;

    while let Some(ch) = chars.next() {
        if ch == '%' && arg_idx < values.len() {
            let val = values[arg_idx];
            if chars.peek() == Some(&'s') {
                chars.next();
                out.push_str(&val.to_string());
                arg_idx += 1;
                continue;
            }
            if chars.peek() == Some(&'d') {
                chars.next();
                out.push_str(&val.to_string());
                arg_idx += 1;
                continue;
            }
            if chars.peek() == Some(&'0') {
                let mut lookahead = chars.clone();
                lookahead.next(); // consume '0'
                let mut width_digits = String::new();
                while let Some(&d) = lookahead.peek() {
                    if d.is_ascii_digit() {
                        width_digits.push(d);
                        lookahead.next();
                    } else {
                        break;
                    }
                }
                if lookahead.peek() == Some(&'d') && !width_digits.is_empty() {
                    let width: usize = width_digits.parse().unwrap_or(2);
                    // Advance main iterator past '0', digits, and 'd'
                    chars.next();
                    for _ in 0..width_digits.len() {
                        chars.next();
                    }
                    chars.next();
                    out.push_str(&format!("{val:0width$}"));
                    arg_idx += 1;
                    continue;
                }
            }
        }
        out.push(ch);
    }

    out
}

/// Resolves and validates the target repository name from push payload and settings.
pub fn resolve_repo_name(settings: &AppSettings, payload: &PushPayload) -> GitUploadResult<String> {
    let repo_name = match payload.mode {
        PushMode::SessionEx => {
            let session: u32 = payload
                .session_val
                .as_deref()
                .unwrap_or("")
                .trim()
                .parse()
                .map_err(|_| {
                    GitUploadError::Config("Session và Exercise phải là số nguyên dương".into())
                })?;
            let exercise: u32 = payload
                .ex_val
                .as_deref()
                .unwrap_or("")
                .trim()
                .parse()
                .map_err(|_| {
                    GitUploadError::Config("Session và Exercise phải là số nguyên dương".into())
                })?;
            if session == 0 || exercise == 0 {
                return Err(GitUploadError::Config(
                    "Session và Exercise phải là số nguyên dương".into(),
                ));
            }
            format_repo_name(&settings.naming_pattern, session, exercise)
        }
        PushMode::MiniProject => {
            let session: u32 = payload
                .session_val
                .as_deref()
                .unwrap_or("")
                .trim()
                .parse()
                .map_err(|_| GitUploadError::Config("Session phải là số nguyên dương".into()))?;
            if session == 0 {
                return Err(GitUploadError::Config(
                    "Session phải là số nguyên dương".into(),
                ));
            }
            let mini_raw = payload.mini_project_val.as_deref().unwrap_or("");
            let sanitized = sanitize_mini_project_text(mini_raw)?;
            format!("session{session:02}-miniproject-{sanitized}")
        }
        PushMode::CustomRepository => {
            let custom = payload
                .custom_repo_name
                .as_deref()
                .unwrap_or("")
                .trim()
                .to_string();
            if custom.is_empty() {
                return Err(GitUploadError::Config(
                    "Tên repository không được để trống".into(),
                ));
            }
            custom
        }
    };

    validate_repository_name(&repo_name)?;
    Ok(repo_name)
}

/// Resolves the commit message for single-file isolation mode with placeholder substitution.
pub fn resolve_commit_msg_for_file(
    settings: &AppSettings,
    use_default: bool,
    custom_msg: Option<&str>,
    file_name: &str,
    file_name_no_ext: &str,
) -> GitUploadResult<String> {
    if !use_default {
        let msg = custom_msg.unwrap_or("").trim();
        if msg.is_empty() {
            return Err(GitUploadError::Config(
                "Commit message tùy chỉnh không được để trống".into(),
            ));
        }
        return Ok(msg.to_string());
    }

    let pattern = if settings.file_commit_msg_pattern.trim().is_empty() {
        "{filename_no_ext}"
    } else {
        settings.file_commit_msg_pattern.as_str()
    };

    let mut msg = pattern
        .replace("{filename}", file_name)
        .replace("{filename_no_ext}", file_name_no_ext);

    if let Some(meta) = parse_file_metadata(file_name) {
        msg = msg
            .replace("{session}", &format!("{:02}", meta.session))
            .replace("{exercise}", &format!("{:02}", meta.exercise));
        if let Some(name) = meta.student_name {
            msg = msg.replace("{name}", &name);
        }
        if let Some(id) = meta.student_id {
            msg = msg.replace("{student_id}", &id);
        }
    }

    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(
            slugify_repo_name("  Session 15 - Ex 02  "),
            "Session-15-Ex-02"
        );
        assert_eq!(slugify_repo_name("..hello---world!!.."), "hello-world");
        assert_eq!(
            slugify_repo_name("feat(ui): add primer theme"),
            "featui-add-primer-theme"
        );
    }

    #[test]
    fn test_file_meta() {
        let meta = parse_file_metadata("SS15_HW02_DucAnh_HE170001.java").unwrap();
        assert_eq!(meta.session, 15);
        assert_eq!(meta.exercise, 2);
        assert_eq!(meta.student_name.as_deref(), Some("DucAnh"));
        assert_eq!(meta.student_id.as_deref(), Some("HE170001"));

        let meta_ex = parse_file_metadata("C:\\Users\\Demo\\ss04-ex09.md").unwrap();
        assert_eq!(meta_ex.session, 4);
        assert_eq!(meta_ex.exercise, 9);
        assert!(meta_ex.student_name.is_none());
    }

    #[test]
    fn test_format_repo_name() {
        assert_eq!(
            format_repo_name("session%02d-ex%02d-homework", 5, 2),
            "session05-ex02-homework"
        );
        assert_eq!(
            format_repo_name("session%s-ex%s-homework", 15, 1),
            "session15-ex1-homework"
        );
        assert_eq!(
            format_repo_name("s{session:02}-e{exercise:02}", 7, 3),
            "s07-e03"
        );
    }

    #[test]
    fn test_validate_repository_name() {
        assert!(validate_repository_name("my-repo_1.0").is_ok());
        assert!(validate_repository_name("").is_err());
        assert!(validate_repository_name(".hidden").is_err());
        assert!(validate_repository_name("bad..repo").is_err());
        assert!(validate_repository_name("bad/repo").is_err());
    }

    #[test]
    fn test_resolve_commit_msg_for_file() {
        let settings = AppSettings {
            file_commit_msg_pattern: "feat(hw): SS{session}-EX{exercise} by {name} ({student_id})"
                .into(),
            ..AppSettings::default()
        };
        let resolved = resolve_commit_msg_for_file(
            &settings,
            true,
            None,
            "SS15_HW02_DucAnh_HE170001.java",
            "SS15_HW02_DucAnh_HE170001",
        )
        .unwrap();
        assert_eq!(resolved, "feat(hw): SS15-EX02 by DucAnh (HE170001)");
    }
}
