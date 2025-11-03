// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

use super::ShellHook;
use anyhow::Result;

pub struct BashHook;

impl ShellHook for BashHook {
    fn generate(&self) -> Result<String> {
        Ok(BASH_HOOK_SCRIPT.to_string())
    }
}

// Embed the bash hook script at compile time
const BASH_HOOK_SCRIPT: &str = include_str!("bash_hook.sh");
