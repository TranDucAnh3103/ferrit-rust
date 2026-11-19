export type ThemeMode =
  | "dark"
  | "dimmed"
  | "git_aurora"
  | "high_contrast"
  | "light"
  | "warm_paper";

export interface ThemeDescriptor {
  id: ThemeMode;
  label: string;
  polarity: "dark" | "light";
  description: string;
  swatches: [string, string, string];
}

export const THEME_OPTIONS: ThemeDescriptor[] = [
  {
    id: "dark",
    label: "GitHub Dark",
    polarity: "dark",
    description: "Primer default dark canvas",
    swatches: ["#0d1117", "#58a6ff", "#3fb950"],
  },
  {
    id: "dimmed",
    label: "Dark Dimmed",
    polarity: "dark",
    description: "Low-glare slate surface",
    swatches: ["#22272e", "#539bf5", "#57ab5a"],
  },
  {
    id: "git_aurora",
    label: "Git Copper Obsidian",
    polarity: "dark",
    description: "Git ember brand + cyan branches",
    swatches: ["#0b0f14", "#f05032", "#38bdf8"],
  },
  {
    id: "high_contrast",
    label: "High Contrast",
    polarity: "dark",
    description: "WCAG AAA crisp contrast",
    swatches: ["#010409", "#71b7ff", "#26cd4d"],
  },
  {
    id: "light",
    label: "GitHub Light",
    polarity: "light",
    description: "Daylight clean Primer canvas",
    swatches: ["#ffffff", "#0969da", "#1a7f37"],
  },
  {
    id: "warm_paper",
    label: "Git Soft Sepia",
    polarity: "light",
    description: "Warm paper studio theme",
    swatches: ["#faf8f5", "#c2410c", "#15803d"],
  },
];

export type ActiveTab =
  | "upload"
  | "graph"
  | "mirror"
  | "merge"
  | "semantic"
  | "branches"
  | "repos"
  | "pulls"
  | "actions"
  | "issues"
  | "ssh"
  | "security"
  | "plugins"
  | "marketplace"
  | "analytics"
  | "schedule"
  | "enterprise"
  | "report"
  | "guide"
  | "settings";

export type PushMode = "SESSION_EX" | "MINI_PROJECT" | "CUSTOM_REPOSITORY";

export interface PushPayload {
  folder_path: string;
  mode: PushMode;
  session_val: string | null;
  ex_val: string | null;
  mini_project_val: string | null;
  custom_repo_name: string | null;
  use_default_commit: boolean;
  custom_commit_msg: string | null;
  is_file: boolean;
  target_org: string | null;
  safe_sync: boolean;
}

export interface PushResult {
  repo_name: string;
  repo_url: string;
  commit_sha: string;
  branch: string;
  files_count: number;
  duration_ms: number;
  pushed_at: string;
}

export interface PushHistoryRecord {
  id: string;
  account_id: string;
  repo_name: string;
  repo_url: string;
  local_path: string;
  commit_sha: string | null;
  commit_msg: string;
  files_count: number;
  status: string;
  error_message: string | null;
  is_force_with_lease: boolean;
  duration_ms: number;
  created_at: string;
}

export interface RepoInspection {
  is_git_repo: boolean;
  current_branch: string | null;
  remote_url: string | null;
  ahead: number;
  behind: number;
  dirty_files: string[];
  has_uncommitted_changes: boolean;
  has_lfs: boolean;
  has_hooks: boolean;
}

export interface GitRefBadge {
  name: string;
  ref_type: "LocalBranch" | "RemoteBranch" | "Tag" | "Head" | "Stash";
  is_head: boolean;
}

export interface GraphLineSegment {
  from_col: number;
  to_col: number;
  from_row: number;
  to_row: number;
  line_type: "Straight" | "BranchOut" | "MergeIn";
  color: string;
}

export interface GitGraphNode {
  oid: string;
  short_sha: string;
  parent_oids: string[];
  author_name: string;
  author_email: string;
  author_avatar_url: string | null;
  timestamp: number;
  summary: string;
  body: string | null;
  column: number;
  color: string;
  lines: GraphLineSegment[];
  refs: GitRefBadge[];
  is_stash: boolean;
  is_merge: boolean;
  signature_verified: boolean;
}

export interface BranchSwimlane {
  index: number;
  branch_name: string | null;
  color: string;
  is_active: boolean;
}

export interface GitDagResponse {
  nodes: GitGraphNode[];
  swimlanes: BranchSwimlane[];
  total_commits: number;
}

export interface GhCliStatus {
  is_installed: boolean;
  is_sandboxed: boolean;
  version: string | null;
  binary_path: string | null;
}

export interface GitHubOrg {
  login: string;
  id: number;
  avatar_url: string;
  description: string | null;
  is_admin: boolean;
  role: "admin" | "member";
  can_create_repo: boolean;
}

export interface RepositoryInfo {
  name: string;
  full_name: string;
  html_url: string;
  clone_url: string;
  size: number;
  description: string | null;
  stargazers_count: number;
  forks_count: number;
  watchers_count: number;
  private: boolean;
  created_at: string;
  updated_at: string;
  pushed_at: string | null;
  default_branch: string | null;
  language: string | null;
}

export interface ContentInfo {
  name: string;
  path: string;
  type: "file" | "dir";
  size: number;
  download_url: string | null;
}

export interface PullRequestInfo {
  id: number;
  number: number;
  title: string;
  body: string | null;
  state: string;
  draft: boolean;
  html_url: string;
  user_login: string;
  head_ref: string;
  base_ref: string;
  mergeable: boolean | null;
  comments_count: number;
  created_at: string;
}

export interface WorkflowRun {
  id: number;
  name: string;
  head_branch: string;
  head_sha: string;
  event: string;
  status: string;
  conclusion: string | null;
  html_url: string;
  created_at: string;
}

export interface IssueSummary {
  id: number;
  number: number;
  title: string;
  body: string | null;
  state: string;
  author: string;
  labels: string[];
  created_at: string;
}

export interface TokenReport {
  scopes: string[];
  rate_limit: number;
  rate_remaining: number;
  rate_reset: string;
  public_repos: number;
  private_repos: number;
  disk_usage: number;
  plan_name: string;
  two_factor_authentication: boolean;
  has_repo_scope: boolean;
  has_delete_scope: boolean;
  has_org_scope: boolean;
  username: string;
  avatar_url: string;
  email: string | null;
}

export interface AppSettings {
  github_username: string;
  github_org_name: string;
  naming_pattern: string;
  theme_mode: ThemeMode;
  git_commit_name: string;
  git_commit_email: string;
  repo_private: boolean;
  auto_push: boolean;
  default_commit_msg: string;
  file_commit_msg_pattern: string;
  sync_file_repo_name_with_commit: boolean;
  cloud_ai_provider: string;
  cloud_ai_model: string;
  language: string;
  safe_sync_enabled: boolean;
  auto_stash_enabled: boolean;
}

export interface InstalledIde {
  name: string;
  path: string;
}

export interface JobProgressPayload {
  job_id: string;
  stage: string;
  percent: number;
  message: string;
}

export interface SanitizedLogLine {
  timestamp: string;
  level: string;
  text: string;
}

export interface JobLogBatchPayload {
  job_id: string;
  lines: SanitizedLogLine[];
}

export interface IpcErrorPayload {
  code: string;
  message: string;
  recoverable: boolean;
}

export function formatIpcError(err: unknown): string {
  if (typeof err === "object" && err !== null && "message" in err) {
    return String((err as IpcErrorPayload).message);
  }
  return String(err);
}

// ============================================================================
// Sprint 2 DTO Contracts (Multi-Remote, 3-Way Merge, SSH Agent, Analytics)
// ============================================================================

export type RemoteProviderType =
  | "github"
  | "gitlab"
  | "bitbucket"
  | "gitea"
  | "custom";

export type SyncPolicy = "bidirectional" | "push_only" | "backup_mirror";

export interface RemoteEndpoint {
  name: string;
  url: string;
  provider: RemoteProviderType;
  sync_policy: SyncPolicy;
  auth_account: string;
  is_enabled: boolean;
  timeout_seconds: number;
}

export type SyncStatus = "success" | "failed" | "timed_out" | "skipped";

export interface RemotePushResult {
  remote_name: string;
  is_success: boolean;
  status: SyncStatus;
  bytes_transferred: number;
  error_message: string | null;
  duration_ms: number;
}

export type RemoteHealthStatus =
  | "healthy"
  | "auth_failed"
  | "timeout"
  | "unreachable";

export interface RemoteHealthReport {
  remote_name: string;
  status: RemoteHealthStatus;
  latency_ms: number;
  message: string;
}

export interface MultiRemoteProgressPayload {
  remotes: Record<string, number>;
  overall_percentage: number;
}

export type ConflictResolutionChoice =
  | { kind: "accept_current" }
  | { kind: "accept_incoming" }
  | { kind: "accept_both" }
  | { kind: "custom"; content: string };

export interface ConflictHunk {
  hunk_id: string;
  file_path: string;
  hunk_index: number;
  start_line_ours: number;
  end_line_ours: number;
  start_line_theirs: number;
  end_line_theirs: number;
  content_ours: string;
  content_theirs: string;
  content_base: string | null;
}

export interface FileConflictDescriptor {
  file_path: string;
  is_binary: boolean;
  total_conflicts: number;
  full_ours: string;
  full_theirs: string;
  full_base: string | null;
  raw_conflicted_text: string;
  hunks: ConflictHunk[];
}

export interface MergeConflictSession {
  repo_path: string;
  branch_ours: string;
  branch_theirs: string;
  ours_oid: string;
  theirs_oid: string;
  conflicting_files: FileConflictDescriptor[];
}

export interface FileResolutionRequest {
  file_path: string;
  chunk_resolutions: [number, ConflictResolutionChoice][];
  custom_content: string | null;
}

export type SshKeyAlgorithm = "ed25519" | "rsa4096";

export interface SshKeyDescriptor {
  key_id: string;
  title: string;
  algorithm: SshKeyAlgorithm;
  public_key: string;
  fingerprint_sha256: string;
  created_at: string;
  is_registered_on_github: boolean;
}

export interface SshAgentStatusPayload {
  is_running: boolean;
  pipe_name: string;
  loaded_key_count: number;
}

export interface HeatmapDay {
  date: string;
  count: number;
  tier: number;
}

export interface VelocityMetrics {
  commits_last_30d: number;
  active_days_30d: number;
  avg_commits_per_active_day: number;
  total_additions: number;
  total_deletions: number;
  churn_ratio: number;
  current_streak: number;
  longest_streak: number;
  on_time_rate: number;
}

export interface ClassroomAssignment {
  id: number;
  session_num: number;
  exercise_num: number;
  title: string;
  deadline_utc: string;
}

export type SubmissionStatus = "on_time" | "late" | "missing";

export interface SubmissionEvaluation {
  session_num: number;
  exercise_num: number;
  title: string;
  status: SubmissionStatus;
  submitted_at: string | null;
}

export interface LeaderboardEntry {
  rank: number;
  author_email: string;
  total_commits: number;
  current_streak: number;
  conventional_ratio: number;
  on_time_count: number;
  score: number;
}

// ============================================================================
// Sprint 3 DTO Contracts (DevSecOps Scanner, Semantic Search, Hygiene, Cron)
// ============================================================================

export type VulnerabilitySeverity =
  | "Critical"
  | "High"
  | "Medium"
  | "Low"
  | "Informational";

export interface SecurityPatchProposal {
  patch_description: string;
  original_code: string;
  replacement_code: string;
  diff_unified: string;
}

export interface VulnerabilityFinding {
  id: string;
  title: string;
  description: string;
  severity: VulnerabilitySeverity;
  cwe_id: string | null;
  owasp_category: string | null;
  file_path: string;
  line_start: number;
  line_end: number;
  vulnerable_snippet: string;
  suggested_fix: SecurityPatchProposal | null;
}

export interface SecurityAuditReport {
  scan_id: string;
  repo_path: string;
  total_findings: number;
  critical_count: number;
  high_count: number;
  medium_count: number;
  low_count: number;
  findings: VulnerabilityFinding[];
  scan_duration_ms: number;
  passed: boolean;
}

export interface SearchHit {
  commit_oid: string;
  summary: string;
  similarity_score: number;
}

export type BranchMergeStatus =
  | "FullyMerged"
  | "SquashedAndMerged"
  | "UnmergedAhead"
  | "StaleInactive";

export interface StaleBranchInfo {
  name: string;
  is_remote: boolean;
  remote_name: string | null;
  last_commit_sha: string;
  last_commit_date: number;
  days_since_last_commit: number;
  author_name: string;
  merge_status: BranchMergeStatus;
  is_protected: boolean;
  associated_pr_number: number | null;
}

export interface PruneBranchesRequest {
  repo_path: string;
  branch_names: string[];
  delete_remote: boolean;
  prune_tracking_refs: boolean;
}

export interface PruneBranchesResult {
  deleted_local_count: number;
  deleted_remote_count: number;
  failed_deletions: [string, string][];
}

export type PowerState = "ac_connected" | "battery_normal" | "battery_low_or_saver";

export interface SyncScheduleConfig {
  enabled: boolean;
  interval_minutes: number;
  allow_on_battery: boolean;
  backup_bundles_enabled: boolean;
}

export type AutoSyncOutcome =
  | { kind: "up_to_date" }
  | { kind: "fast_forwarded"; commits_pulled: number }
  | { kind: "diverged"; ahead: number; behind: number };

// ============================================================================
// Sprint 4 DTO Contracts (Auto-Updater, Enterprise Air-Gap, Proxy, Platform)
// ============================================================================

export type UpdateChannel = "stable" | "beta" | "nightly";

export interface UpdateManifestInfo {
  version: string;
  body: string;
  date: string;
  download_url: string;
  signature_verified: boolean;
}

export interface EffectiveEnterprisePolicy {
  organization_name: string;
  gpo_locks_cloud_ai: boolean;
  gpo_locks_airgap: boolean;
  airgap_enforced: boolean;
  cloud_ai_disabled: boolean;
  telemetry_disabled: boolean;
  corporate_proxy_url: string | null;
  allowed_remotes: string[];
  enforced_commit_sign: boolean;
}

export interface PlatformCapabilitiesInfo {
  os_name: string;
  arch: string;
  vault_backend: string;
  policy: EffectiveEnterprisePolicy;
}

export interface CorporateProxyConfig {
  proxy_url: string;
  custom_ca_cert_path: string | null;
  bypass_hosts: string[];
}

// ============================================================================
// Sprint 5 DTO Contracts (WASM Plugins, Webhooks, Marketplace, Headless CLI)
// ============================================================================

export interface PluginCapabilities {
  git_read: boolean;
  git_write: boolean;
  commit_hook: boolean;
  ai_prompt_filter: boolean;
  network_access: boolean;
  max_memory_mb: number;
}

export interface UserPluginRecord {
  id: string;
  name: string;
  version: string;
  author: string;
  entrypoint: string;
  capabilities: PluginCapabilities;
  is_enabled: boolean;
  installed_at: string;
}

export type PluginCategory =
  | "linter"
  | "ai_prompt"
  | "ui_theme"
  | "webhook"
  | "automation";

export interface MarketplacePluginEntry {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  category: PluginCategory;
  download_url: string;
  sha256: string;
  ed25519_signature: string;
  publisher_pubkey: string;
  min_app_version: string;
  capabilities: string[];
  verified: boolean;
  downloads_count: number;
}

export interface MarketplaceIndex {
  version: number;
  updated_at: string;
  plugins: MarketplacePluginEntry[];
}

export type WebhookTransport = "discord" | "slack" | "telegram" | "generic";

export interface WebhookEndpoint {
  id: string;
  name: string;
  transport: WebhookTransport;
  target_url: string;
  vault_secret_key?: string | null;
  events: string[];
  is_active: boolean;
  created_at: string;
}

export interface WebhookDeliveryRecord {
  id: string;
  endpoint_id: string;
  event_type: string;
  status_code?: number | null;
  attempt_count: number;
  duration_ms: number;
  error_message?: string | null;
  delivered_at: string;
}
