//! GitHub Organization and multi-account profile domain models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Role held by the authenticated user within a GitHub Organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrgMembershipRole {
    /// Organization owner / administrator.
    Admin,
    /// Standard organization member.
    Member,
}

/// GitHub Organization metadata and repository creation permissions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubOrg {
    /// Organization login slug (e.g. `"acme-corp"`).
    pub login: String,
    /// Numeric GitHub organization ID.
    pub id: u64,
    /// Avatar URL.
    pub avatar_url: String,
    /// Optional organization description.
    pub description: Option<String>,
    /// True when the user has administrative privileges.
    pub is_admin: bool,
    /// Effective membership role.
    pub role: OrgMembershipRole,
    /// True when the user is permitted to create repositories inside the organization.
    pub can_create_repo: bool,
}

/// Account profile record stored in SQLite `accounts` (tokens stored solely in OS Keyring).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountProfile {
    /// UUID v4 string identifier.
    pub id: String,
    /// User-assigned label (e.g. `"Personal"`, `"University"`).
    pub label: String,
    /// GitHub login username.
    pub username: String,
    /// Optional GitHub avatar URL.
    pub avatar_url: Option<String>,
    /// Whether this account is active by default.
    pub is_default: bool,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}
