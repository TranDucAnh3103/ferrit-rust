use gitupload_core::webhooks::WebhookEvent;
use serde_json::json;

/// Formats a WebhookEvent into a Slack Block Kit JSON payload.
pub fn format_slack_payload(event: &WebhookEvent) -> serde_json::Value {
    match event {
        WebhookEvent::GitPush(push) => {
            let short_hash = if push.commit_hash.len() >= 7 {
                &push.commit_hash[..7]
            } else {
                &push.commit_hash
            };
            let mut blocks = vec![
                json!({
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": format!("New Push to {} [{}]", push.repository, push.branch),
                        "emoji": true
                    }
                }),
                json!({
                    "type": "section",
                    "fields": [
                        { "type": "mrkdwn", "text": format!("*Commit:*\n`{}` {}", short_hash, push.commit_message) },
                        { "type": "mrkdwn", "text": format!("*Author:*\n{}", push.author_name) },
                        { "type": "mrkdwn", "text": format!("*Files Changed:*\n{}", push.files_changed) },
                        { "type": "mrkdwn", "text": format!("*Timestamp:*\n{}", push.timestamp.to_rfc3339()) }
                    ]
                }),
            ];

            if let Some(url) = &push.github_url {
                blocks.push(json!({
                    "type": "actions",
                    "elements": [
                        {
                            "type": "button",
                            "text": { "type": "plain_text", "text": "View Commit", "emoji": true },
                            "url": url,
                            "style": "primary"
                        }
                    ]
                }));
            }

            json!({ "blocks": blocks })
        }
        WebhookEvent::PrCreated(pr) => {
            json!({
                "blocks": [
                    {
                        "type": "header",
                        "text": {
                            "type": "plain_text",
                            "text": format!("Pull Request #{}: {}", pr.pr_number, pr.pr_title),
                            "emoji": true
                        }
                    },
                    {
                        "type": "section",
                        "fields": [
                            { "type": "mrkdwn", "text": format!("*Repository:*\n{}", pr.repository) },
                            { "type": "mrkdwn", "text": format!("*Author:*\n{}", pr.author) },
                            { "type": "mrkdwn", "text": format!("*Status:*\n{}", if pr.is_draft { "Draft" } else { "Open" }) }
                        ]
                    },
                    {
                        "type": "actions",
                        "elements": [
                            {
                                "type": "button",
                                "text": { "type": "plain_text", "text": "View on GitHub", "emoji": true },
                                "url": pr.pr_url,
                                "style": "primary"
                            }
                        ]
                    }
                ]
            })
        }
        WebhookEvent::GitSyncFailed(fail) => {
            json!({
                "blocks": [
                    {
                        "type": "header",
                        "text": {
                            "type": "plain_text",
                            "text": format!("Sync Failed: {} [{}]", fail.repository, fail.branch),
                            "emoji": true
                        }
                    },
                    {
                        "type": "section",
                        "text": {
                            "type": "mrkdwn",
                            "text": format!("```\n{}\n```", fail.error_message)
                        }
                    }
                ]
            })
        }
        WebhookEvent::BatchCompleted(batch) => {
            json!({
                "blocks": [
                    {
                        "type": "header",
                        "text": {
                            "type": "plain_text",
                            "text": "Batch Multi-Repository Execution Completed",
                            "emoji": true
                        }
                    },
                    {
                        "type": "section",
                        "fields": [
                            { "type": "mrkdwn", "text": format!("*Total:*\n{}", batch.total_repos) },
                            { "type": "mrkdwn", "text": format!("*Successful:*\n{}", batch.successful_repos) },
                            { "type": "mrkdwn", "text": format!("*Failed:*\n{}", batch.failed_repos) },
                            { "type": "mrkdwn", "text": format!("*Duration:*\n{}ms", batch.duration_ms) }
                        ]
                    }
                ]
            })
        }
    }
}
