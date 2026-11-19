//! GitHub Organizations discovery and permission verification (`/user/orgs`, `/orgs/{org}`).

use crate::client::GitHubClient;
use gitupload_core::errors::GitHubResult;
use gitupload_core::models::{GitHubOrg, OrgMembershipRole};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RawOrg {
    login: String,
    id: u64,
    avatar_url: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawMembership {
    role: Option<String>,
    state: Option<String>,
}

impl GitHubClient {
    /// Lists all GitHub Organizations affiliated with the authenticated user along with role metadata.
    pub async fn list_user_orgs(&self) -> GitHubResult<Vec<GitHubOrg>> {
        let url = format!("{}/user/orgs?per_page=100", self.base_url);
        let raw_orgs: Vec<RawOrg> = self.get(&url).await?;

        let mut orgs = Vec::with_capacity(raw_orgs.len());
        for org in raw_orgs {
            let role = self
                .resolve_org_role(&org.login)
                .await
                .unwrap_or(OrgMembershipRole::Member);
            let is_admin = role == OrgMembershipRole::Admin;
            orgs.push(GitHubOrg {
                login: org.login,
                id: org.id,
                avatar_url: org.avatar_url,
                description: org.description,
                is_admin,
                role,
                can_create_repo: true,
            });
        }

        Ok(orgs)
    }

    async fn resolve_org_role(&self, org_login: &str) -> GitHubResult<OrgMembershipRole> {
        let url = format!("{}/user/memberships/orgs/{}", self.base_url, org_login);
        let membership: RawMembership = self.get(&url).await?;
        let active = membership.state.as_deref() == Some("active");
        if active && membership.role.as_deref() == Some("admin") {
            Ok(OrgMembershipRole::Admin)
        } else {
            Ok(OrgMembershipRole::Member)
        }
    }
}
