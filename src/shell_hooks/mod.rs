// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;

pub mod bash;

pub trait ShellHook {
    fn generate(&self) -> Result<String>;
}

pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bash" => Some(Shell::Bash),
            "zsh" => Some(Shell::Zsh),
            "fish" => Some(Shell::Fish),
            _ => None,
        }
    }

    pub fn get_hook(&self) -> Box<dyn ShellHook> {
        match self {
            Shell::Bash => Box::new(bash::BashHook),
            Shell::Zsh => Box::new(bash::BashHook), // Zsh can use bash hooks
            Shell::Fish => Box::new(bash::BashHook), // TODO: Implement fish hook
        }
    }
}
