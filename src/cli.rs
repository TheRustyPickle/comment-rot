use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "rot",
    version,
    about = "Find comments that have gone stale relative to the code they describe"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Take the first snapshot of comment/code for a project.
    Init {
        #[arg(long, short, default_value = ".")]
        path: PathBuf,
        /// Re-do the snapshot from scratch even if a snapshot already exists.
        #[arg(long)]
        force: bool,
    },
    /// Scan for code that changed without its comment being updated.
    Check {
        #[arg(long, short, default_value = ".")]
        path: PathBuf,
        /// Print candidates as a JSON array on stdout instead of prompting
        /// interactively
        #[arg(long)]
        json: bool,
    },
    /// Apply verdicts for candidates.
    /// Reads a JSON array of `{"id": "...", "verdict": "yes"|"no"}` from
    /// stdin and prints a JSON summary to stdout.
    Confirm {
        #[arg(long, short, default_value = ".")]
        path: PathBuf,
    },
    /// List comments confirmed stale but not yet fixed.
    Status {
        #[arg(long, short, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Manually clear a known-issue entry by id
    Resolve {
        #[arg(long, short, default_value = ".")]
        path: PathBuf,
        id: String,
    },
}
