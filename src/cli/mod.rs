// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "shwrap")]
#[command(about = "A profile manager for Bubblewrap (bwrap)", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub subject: Subject,
}

#[derive(Subcommand)]
pub enum Subject {
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Command management
    Command {
        #[command(subcommand)]
        action: CommandAction,
    },

    /// Shell integration
    #[command(name = "shell-hook")]
    ShellHook {
        #[command(subcommand)]
        action: ShellHookAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Initialize a new .shwrap file with templates
    Init {
        /// Template to use (nodejs, python, ruby, go, rust)
        #[arg(short, long)]
        template: Option<String>,
    },

    /// Validate configuration syntax
    Check {
        /// Path to config file (defaults to searching hierarchy)
        path: Option<String>,
        /// To enable no output (useful for shell exit code returns)
        #[arg(long)]
        silent: bool,
    },

    /// Show which .shwrap file would be used
    Which,
}

#[derive(Subcommand)]
pub enum CommandAction {
    /// List active profiles and configurations
    List {
        /// To enable simple output (useful for shell inputs)
        #[arg(long)]
        simple: bool,
    },

    /// Manually wrap and execute a command
    Exec {
        /// Command to execute
        command: String,

        /// Arguments to pass to the command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Show the bwrap command that would be executed
    Show {
        /// Command to show
        command: String,

        /// Arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum ShellHookAction {
    /// Get shell integration code
    Get {
        /// Shell name
        shell: String,
    },
}
