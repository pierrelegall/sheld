// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sheld")]
#[command(about = "A profile manager for Bubblewrap (bwrap)", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub action: Action,
}

#[derive(Subcommand)]
pub enum Action {
    /// Initialize a new .sheld.yaml file
    Init,

    /// List active profiles and configurations
    List {
        /// To enable simple output (useful for shell inputs)
        #[arg(long)]
        simple: bool,
    },

    /// Manually wrap and execute a command
    Wrap {
        /// Command to execute
        command: String,

        /// Arguments to pass to the command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Execute a command without sandboxing (bypass hook system)
    Bypass {
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

    /// Validate configuration syntax
    Validate {
        /// Path to config file (defaults to searching hierarchy)
        path: Option<String>,
        /// To enable no output (useful for shell exit code returns)
        #[arg(long)]
        silent: bool,
    },

    /// Get shell integration code for activation
    Activate {
        /// Shell name (bash, zsh, fish)
        shell: String,
    },

    /// Check if a command exists in configuration
    Check {
        /// Command name to check
        command: String,

        /// Suppress output (useful for exit code checking in scripts)
        #[arg(long)]
        silent: bool,
    },
}
