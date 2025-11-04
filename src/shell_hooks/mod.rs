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
    Nushell,
}

impl Shell {
    pub fn to_str(&self) -> &str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
            Shell::Nushell => "nushell",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bash" => Some(Shell::Bash),
            "zsh" => Some(Shell::Zsh),
            "fish" => Some(Shell::Fish),
            "nushell" => Some(Shell::Nushell),
            _ => None,
        }
    }

    pub fn get_hook(&self) -> Option<Box<dyn ShellHook>> {
        match self {
            Shell::Bash => Some(Box::new(bash::BashHook)),
            _ => None,
        }
    }
}
