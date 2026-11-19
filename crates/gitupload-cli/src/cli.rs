use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "gitupload",
    author = "GitUpload Engineering Team",
    version = "2.6.0",
    about = "Blazing-fast GitHub-fidelity git management and AI automation CLI",
    long_about = "GitUpload CLI provides headless, scriptable git operations, AI commit message generation, multi-repo batch execution, and GitHub integration with sub-10ms startup."
)]
pub struct Cli {
    #[arg(
        short,
        long,
        global = true,
        help = "Output machine-readable JSON format"
    )]
    pub json: bool,

    #[arg(short, long, global = true, help = "Silence non-error output")]
    pub quiet: bool,

    #[arg(
        short,
        long,
        global = true,
        help = "Repository path (defaults to current directory)"
    )]
    pub repo: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Display detailed repository status, branch tracking, and staged files")]
    Status(StatusArgs),

    #[command(about = "Stage files, commit, and push to upstream remote")]
    Push(PushArgs),

    #[command(about = "Synchronize changes with upstream remote (fetch, pull/rebase)")]
    Sync(SyncArgs),

    #[command(about = "Execute operations across multiple repositories concurrently")]
    Batch(BatchArgs),

    #[command(about = "Inspect and interact with GitHub Pull Requests")]
    Pr(PrArgs),

    #[command(about = "Manage GitUpload configuration, profiles, and credentials")]
    Config(ConfigArgs),

    #[command(about = "Generate shell auto-completion scripts")]
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Args, Debug)]
pub struct StatusArgs {
    #[arg(short, long, help = "Give the output in the short-format")]
    pub short: bool,
}

#[derive(Args, Debug)]
pub struct PushArgs {
    #[arg(short, long, help = "Commit message (if staged changes exist)")]
    pub message: Option<String>,

    #[arg(long, help = "Use Cloud AI to generate conventional commit message")]
    pub ai: bool,

    #[arg(short, long, help = "Target branch (defaults to current branch)")]
    pub branch: Option<String>,

    #[arg(long, help = "Bypass pre-push hooks and WASM linters")]
    pub no_verify: bool,

    #[arg(long, help = "Push with lease")]
    pub force_with_lease: bool,
}

#[derive(Args, Debug)]
pub struct SyncArgs {
    #[arg(long, help = "Rebase local commits instead of creating merge commit")]
    pub rebase: bool,

    #[arg(long, help = "Automatically stash and pop uncommitted changes")]
    pub autostash: bool,
}

#[derive(Args, Debug)]
pub struct BatchArgs {
    #[arg(value_enum, help = "Action to execute across repositories")]
    pub action: BatchAction,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Comma-separated list of repository paths"
    )]
    pub paths: Option<Vec<PathBuf>>,

    #[arg(long, help = "Target all repositories registered in GitUpload desktop")]
    pub all_tracked: bool,

    #[arg(
        short,
        long,
        default_value_t = 3,
        help = "Concurrency limit (max parallel workers)"
    )]
    pub concurrency: usize,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchAction {
    Status,
    Sync,
    Push,
}

#[derive(Args, Debug)]
pub struct PrArgs {
    #[command(subcommand)]
    pub action: PrSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum PrSubcommands {
    #[command(about = "List open pull requests")]
    List,
    #[command(about = "Create a new pull request on GitHub")]
    Create {
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        body: Option<String>,
        #[arg(long)]
        draft: bool,
        #[arg(long)]
        ai_description: bool,
    },
    #[command(about = "Checkout a pull request branch locally")]
    Checkout { pr_number: u64 },
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum ConfigSubcommands {
    List,
    Get { key: String },
    Set { key: String, value: String },
    SwitchProfile { profile_name: String },
}
