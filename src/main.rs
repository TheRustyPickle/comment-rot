mod cli;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use std::process::ExitCode;

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { path, force } => {
            ui::interactive::run_init(&path, force)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Check { path, json } => {
            if json {
                ui::json::run_check(&path)?;
            } else {
                ui::interactive::run_check(&path)?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Confirm { path } => {
            ui::json::run_confirm(&path)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Status { path, json } => {
            if json {
                ui::json::run_status(&path)?;
            } else {
                ui::interactive::run_status(&path)?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Resolve { path, id, json } => {
            let resolved = if json {
                ui::json::run_resolve(&path, &id)?
            } else {
                ui::interactive::run_resolve(&path, &id)?
            };
            Ok(if resolved {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
    }
}
