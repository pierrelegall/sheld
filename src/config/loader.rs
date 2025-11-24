// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;

use super::Config;

/// Local config file name
const LOCAL_CONFIG_FILE_NAME: &str = ".shwrap.yaml";

/// User config file name
const USER_CONFIG_FILE_NAME: &str = "default.yaml";

/// User config directory path relative to HOME
const USER_CONFIG_DIR_PATH: &str = "~/.config/shwrap";

pub struct ConfigLoader;

impl ConfigLoader {
    /// Get the local config file name
    pub fn local_config_name() -> &'static str {
        LOCAL_CONFIG_FILE_NAME
    }

    /// Get the user config file name
    pub fn user_config_name() -> &'static str {
        USER_CONFIG_FILE_NAME
    }

    /// Get the user config directory path (constant, not expanded)
    pub fn user_config_dir() -> &'static str {
        USER_CONFIG_DIR_PATH
    }

    /// Get the directory containing the local config file by walking up from current directory
    /// Returns None if no directory contains a local config file
    pub fn get_local_config_dir() -> Result<Option<PathBuf>> {
        let current_dir = env::current_dir().context("Failed to get current directory")?;
        let mut dir = current_dir.as_path();

        loop {
            let config_path = dir.join(LOCAL_CONFIG_FILE_NAME);
            if config_path.exists() {
                return Ok(Some(dir.to_path_buf()));
            }

            // Move to parent directory
            match dir.parent() {
                Some(parent) => dir = parent,
                None => break,
            }
        }

        Ok(None)
    }

    /// Get the user config directory (expanded) path
    pub fn get_user_config_dir() -> PathBuf {
        let expanded_dir = shellexpand::tilde(USER_CONFIG_DIR_PATH);
        PathBuf::from(expanded_dir.as_ref())
    }

    /// Get config file path in hierarchical order (local first, then user)
    pub fn get_config_file() -> Result<Option<PathBuf>> {
        // Look for local config in current directory and parent directories
        if let Some(local_config) = Self::get_local_config_file()? {
            return Ok(Some(local_config));
        }

        // Look for user-level config
        if let Some(user_config) = Self::get_user_config_file()? {
            return Ok(Some(user_config));
        }

        Ok(None)
    }

    /// Get local config file by searching in current and parent directories
    pub fn get_local_config_file() -> Result<Option<PathBuf>> {
        if let Some(dir) = Self::get_local_config_dir()? {
            let config_path = dir.join(LOCAL_CONFIG_FILE_NAME);
            return Ok(Some(config_path));
        }

        Ok(None)
    }

    /// Get user-level config file
    pub fn get_user_config_file() -> Result<Option<PathBuf>> {
        let config_path = Self::get_user_config_dir().join(USER_CONFIG_FILE_NAME);

        if config_path.exists() {
            return Ok(Some(config_path));
        }

        Ok(None)
    }

    /// Load config from the found path
    /// If both user and local configs exist, merge them (local overrides user)
    pub fn load() -> Result<Option<Config>> {
        let user_config = Self::get_user_config_file()?;
        let local_config = Self::get_local_config_file()?;

        match (user_config, local_config) {
            (Some(user_path), Some(local_path)) => {
                // Both exist: merge them (local overrides user)
                let user = Config::from_file(&user_path)?;
                let local = Config::from_file(&local_path)?;
                Ok(Some(Config::merge(user, local)))
            }
            (Some(user_path), None) => {
                // Only user config exists
                let config = Config::from_file(&user_path)?;
                Ok(Some(config))
            }
            (None, Some(local_path)) => {
                // Only local config exists
                let config = Config::from_file(&local_path)?;
                Ok(Some(config))
            }
            (None, None) => {
                // No config exists
                Ok(None)
            }
        }
    }
}
