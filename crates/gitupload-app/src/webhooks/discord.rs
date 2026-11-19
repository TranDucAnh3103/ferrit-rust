use gitupload_core::webhooks::WebhookEvent;
use serde_json::json;

/// Formats a WebhookEvent into a Discord Rich Embed JSON payload matching GitHub Primer palette.
pub fn format_discord_payload(event: &WebhookEvent) -> serde_json::Value {
    match event {
        WebhookEvent::GitPush(push) => {
            let short_hash = if push.commit_hash.len() >= 7 {
                &push.commit_hash[..7]
            } else {
                &push.commit_hash
            };
            let commit_link = push.github_url.clone().unwrap_or_else(|| "#".to_string());

            json!({
                "username": "GitUpload Desktop",
                "avatar_url": "https://gitupload.org/assets/avatar.png",
                "embeds": [{
                    "title": format!("New Push to {} [{}]", push.repository, push.branch),
                    "description": format!("`{}` - {}", short_hash, push.commit_message),
                    "url": commit_link,
                    "color": 0x2DA44E, // Primer Green
                    "fields": [
                        { "name": "Author", "value": &push.author_name, "inline": true },
                        { "name": "Files Changed", "value": push.files_changed.to_string(), "inline": true }
                    ],
                    "timestamp": push.timestamp.to_rfc3339()
                }]
            })
        }
        WebhookEvent::GitSyncFailed(fail) => {
            json!({
                "username": "GitUpload Desktop",
                "avatar_url": "https://gitupload.org/assets/avatar.png",
                "embeds": [{
                    "title": format!("Sync Failure in {} [{}]", fail.repository, fail.branch),
                    "description": format!("```\n{}\n```", fail.error_message),
                    "color": 0xCF222E, // Primer Red
                    "timestamp": fail.timestamp.to_rfc3339()
                }]
            })
        }
        WebhookEvent::PrCreated(pr) => {
            json!({
                "username": "GitUpload Desktop",
                "avatar_url": "https://gitupload.org/assets/avatar.png",
                "embeds": [{
                    "title": format!("Pull Request #{}: {}", pr.pr_number, pr.pr_title),
                    "url": &pr.pr_url,
                    "color": 0x8250DF, // Primer Purple
                    "fields": [
                        { "name": "Repository", "value": &pr.repository, "inline": true },
                        { "name": "Author", "value": &pr.author, "inline": true },
                        { "name": "Status", "value": if pr.is_draft { "Draft" } else { "Ready for Review" }, "inline": true }
                    ],
                    "timestamp": pr.timestamp.to_rfc3339()
                }]
            })
        }
        WebhookEvent::BatchCompleted(batch) => {
            json!({
                "username": "GitUpload Desktop",
                "avatar_url": "https://gitupload.org/assets/avatar.png",
                "embeds": [{
                    "title": "Batch Multi-Repository Execution Completed",
                    "color": 0x0969DA, // Primer Blue
                    "fields": [
                        { "name": "Total Repositories", "value": batch.total_repos.to_string(), "inline": true },
                        { "name": "Successful", "value": batch.successful_repos.to_string(), "inline": true },
                        { "name": "Failed", "value": batch.failed_repos.to_string(), "inline": true },
                        { "name": "Duration", "value": format!("{}ms", batch.duration_ms), "inline": true }
                    ],
                    "timestamp": batch.timestamp.to_rfc3339()
                }]
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use gitupload_core::webhooks::GitPushPayload;

    #[test]
    fn test_format_discord_push() {
        let event = WebhookEvent::GitPush(GitPushPayload {
            repository: "my-org/my-repo".into(),
            branch: "main".into(),
            commit_hash: "abcdef123456789".into(),
            commit_message: "feat: add webhook support".into(),
            author_name: "Stephen".into(),
            author_email: "test@example.com".into(),
            files_changed: 4,
            timestamp: Utc::now(),
            github_url: Some("https://github.com/my-org/my-repo/commit/abcdef1".into()),
        });

        let payload = format_discord_payload(&event);
        assert_eq!(payload["username"], "GitUpload Desktop");
        assert_eq!(payload["embeds"][0]["color"], 0x2DA44E);
    }
}
