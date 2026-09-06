mod cli;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use log::LevelFilter;

fn main() -> Result<()> {
    pretty_env_logger::formatted_timed_builder()
        .format_timestamp_millis()
        .filter_module(env!("CARGO_BIN_NAME"), LevelFilter::Info)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Init { path, force } => ui::interactive::run_init(&path, force),
        Command::Check { path, json } => {
            if json {
                ui::json::run_check(&path)
            } else {
                ui::interactive::run_check(&path)
            }
        }
        Command::Confirm { path } => ui::json::run_confirm(&path),
        Command::Status { path, json } => {
            if json {
                ui::json::run_status(&path)
            } else {
                ui::interactive::run_status(&path)
            }
        }
        Command::Resolve { path, id } => ui::interactive::run_resolve(&path, &id),
    }
}
