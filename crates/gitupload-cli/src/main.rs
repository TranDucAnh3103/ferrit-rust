#![forbid(unsafe_code)]

//! Standalone headless companion CLI tool for GitUpload Desktop.
//! Sub-10ms startup, scriptable JSON output, and batch automation.

mod cli;
mod commands;
mod output;

use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Commands::Status(status_args) => {
            commands::status::execute_status(&status_args, args.repo.as_deref(), args.json)?;
        }
        Commands::Completions { shell } => {
            commands::completions::execute_completions(shell);
        }
        Commands::Config(config_args) => {
            commands::config::execute_config(&config_args, args.json)?;
        }
        Commands::Sync(sync_args) => {
            commands::sync::execute_sync(&sync_args, args.repo.as_deref(), args.json, args.quiet)?;
        }
        Commands::Push(push_args) => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            rt.block_on(commands::push::execute_push(
                &push_args,
                args.repo.as_deref(),
                args.json,
                args.quiet,
            ))?;
        }
        Commands::Batch(batch_args) => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            rt.block_on(commands::batch::execute_batch(
                &batch_args,
                args.json,
                args.quiet,
            ))?;
        }
        Commands::Pr(pr_args) => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            rt.block_on(commands::pr::execute_pr(
                &pr_args,
                args.repo.as_deref(),
                args.json,
            ))?;
        }
    }

    Ok(())
}
