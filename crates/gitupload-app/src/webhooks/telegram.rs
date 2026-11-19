use gitupload_core::webhooks::WebhookEvent;
use serde_json::json;

/// Escapes Telegram MarkdownV2 reserved characters.
/// Specifically: `_`, `*`, `[`, `]`, `(`, `)`, `~`, `` ` ``, `>`, `#`, `+`, `-`, `=`, `|`, `{`, `}`, `.`, `!`
pub fn escape_markdown_v2(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    for ch in text.chars() {
        match ch {
            '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '=' | '|'
            | '{' | '}' | '.' | '!' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Formats a WebhookEvent into a Telegram Bot API `sendMessage` JSON payload.
pub fn format_telegram_payload(event: &WebhookEvent, chat_id: &str) -> serde_json::Value {
    let text = match event {
        WebhookEvent::GitPush(push) => {
            let short_hash = if push.commit_hash.len() >= 7 {
                &push.commit_hash[..7]
            } else {
                &push.commit_hash
            };
            format!(
                "*New Push to {} \\[{sub_branch}\\]*\n`{hash}` {msg}\nAuthor: {author}\nFiles Changed: {files}",
                escape_markdown_v2(&push.repository),
                sub_branch = escape_markdown_v2(&push.branch),
                hash = escape_markdown_v2(short_hash),
                msg = escape_markdown_v2(&push.commit_message),
                author = escape_markdown_v2(&push.author_name),
                files = push.files_changed
            )
        }
        WebhookEvent::GitSyncFailed(fail) => {
            format!(
                "*Sync Failure in {} \\[{sub_branch}\\]*\n```\n{err}\n```",
                escape_markdown_v2(&fail.repository),
                sub_branch = escape_markdown_v2(&fail.branch),
                err = escape_markdown_v2(&fail.error_message)
            )
        }
        WebhookEvent::PrCreated(pr) => {
            format!(
                "*Pull Request #{num}: {title}*\nRepository: {repo}\nAuthor: {author}\nStatus: {status}\n[View on GitHub]({url})",
                num = pr.pr_number,
                title = escape_markdown_v2(&pr.pr_title),
                repo = escape_markdown_v2(&pr.repository),
                author = escape_markdown_v2(&pr.author),
                status = if pr.is_draft { "Draft" } else { "Open" },
                url = escape_markdown_v2(&pr.pr_url)
            )
        }
        WebhookEvent::BatchCompleted(batch) => {
            format!(
                "*Batch Completed*\nTotal: {total}\nSuccess: {succ}\nFailed: {fail}\nDuration: {dur}ms",
                total = batch.total_repos,
                succ = batch.successful_repos,
                fail = batch.failed_repos,
                dur = batch.duration_ms
            )
        }
    };

    json!({
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "MarkdownV2"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_markdown_v2() {
        let raw = "fix(auth): update v1.2 -> v1.3! #test [WIP]";
        let escaped = escape_markdown_v2(raw);
        assert_eq!(
            escaped,
            "fix\\(auth\\): update v1\\.2 \\-\\> v1\\.3\\! \\#test \\[WIP\\]"
        );
    }
}
