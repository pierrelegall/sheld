// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

const BASH_HOOK: &str = include_str!("hook.bash");
const ZSH_HOOK: &str = include_str!("hook.zsh");
const FISH_HOOK: &str = include_str!("hook.fish");

pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    pub fn to_str(&self) -> &str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bash" => Some(Shell::Bash),
            "zsh" => Some(Shell::Zsh),
            "fish" => Some(Shell::Fish),
            _ => None,
        }
    }

    pub fn get_hook(&self) -> Option<&str> {
        match self {
            Shell::Bash => Some(BASH_HOOK),
            Shell::Zsh => Some(ZSH_HOOK),
            Shell::Fish => Some(FISH_HOOK),
        }
    }
}
