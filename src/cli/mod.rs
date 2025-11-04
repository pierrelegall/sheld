// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "shwrap")]
#[command(about = "A profile manager for Bubblewrap (bwrap)", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new .shwrap file with templates
    Init {
        /// Template to use (nodejs, python, ruby, go, rust)
        #[arg(short, long)]
        template: Option<String>,
    },

    /// Manually wrap and execute a command
    Exec {
        /// Command to execute
        command: String,

        /// Arguments to pass to the command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Validate configuration syntax
    Check {
        /// Path to config file (defaults to searching hierarchy)
        path: Option<String>,
    },

    /// List active profiles and configurations
    List,

    /// Show which .shwrap file would be used
    Which,

    /// Show the bwrap command that would be executed
    Show {
        /// Command to show
        command: String,

        /// Arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Output shell integration code
    ShellHook {
        /// Shell type (bash, zsh, fish)
        #[arg(default_value = "bash")]
        shell: String,
    },
}
