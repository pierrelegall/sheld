// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};

use super::BwrapConfig;

pub struct ConfigLoader;

impl ConfigLoader {
    /// Search for .shwrap config file in hierarchical order
    pub fn find_config() -> Result<Option<PathBuf>> {
        // 1. Look for .shwrap in current directory and parent directories
        if let Some(local_config) = Self::find_local_config()? {
            return Ok(Some(local_config));
        }

        // 2. Look for user-level config
        if let Some(user_config) = Self::find_user_config()? {
            return Ok(Some(user_config));
        }

        Ok(None)
    }

    /// Find .shwrap file in current or parent directories
    pub fn find_local_config() -> Result<Option<PathBuf>> {
        let current_dir = env::current_dir().context("Failed to get current directory")?;

        let mut dir = current_dir.as_path();

        loop {
            let config_path = dir.join(".shwrap");
            if config_path.exists() {
                return Ok(Some(config_path));
            }

            // Move to parent directory
            match dir.parent() {
                Some(parent) => dir = parent,
                None => break,
            }
        }

        Ok(None)
    }

    /// Find user-level config at ~/.config/shwrap/config
    pub fn find_user_config() -> Result<Option<PathBuf>> {
        if let Some(home) = env::var_os("HOME") {
            let config_path = Path::new(&home)
                .join(".config")
                .join("shwrap")
                .join("config");

            if config_path.exists() {
                return Ok(Some(config_path));
            }
        }

        Ok(None)
    }

    /// Load config from the found path
    pub fn load() -> Result<Option<BwrapConfig>> {
        if let Some(path) = Self::find_config()? {
            let config = BwrapConfig::from_file(&path)?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }
}
