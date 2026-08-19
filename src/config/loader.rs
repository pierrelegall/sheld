// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};

use super::Config;

/// Local config file name
const LOCAL_CONFIG_FILE_NAME: &str = ".sheld.yaml";

/// User config file name
const USER_CONFIG_FILE_NAME: &str = "default.yaml";

/// User config directory path relative to HOME (for display)
const USER_CONFIG_DIR_PATH: &str = "~/.config/sheld";

/// User config subdirectory relative to home
const USER_CONFIG_DIR_RELATIVE: &str = ".config/sheld";

pub struct ConfigLoader;

impl ConfigLoader {
    // --- Constants ---

    pub fn local_config_name() -> &'static str {
        LOCAL_CONFIG_FILE_NAME
    }

    pub fn user_config_name() -> &'static str {
        USER_CONFIG_FILE_NAME
    }

    pub fn user_config_dir() -> &'static str {
        USER_CONFIG_DIR_PATH
    }

    // --- Core logic (explicit paths, no global state) ---

    /// Walk up from `start` to find the directory containing a local config file.
    pub fn get_local_config_dir_from(start: &Path) -> Option<PathBuf> {
        let mut dir = start;
        loop {
            let config_path = dir.join(LOCAL_CONFIG_FILE_NAME);
            if config_path.exists() {
                return Some(dir.to_path_buf());
            }
            match dir.parent() {
                Some(parent) => dir = parent,
                None => break,
            }
        }
        None
    }

    /// Derive the local config file path by walking up from `start`.
    pub fn get_local_config_file_from(start: &Path) -> Option<PathBuf> {
        Self::get_local_config_dir_from(start).map(|dir| dir.join(LOCAL_CONFIG_FILE_NAME))
    }

    /// Build the user config directory from an explicit home path.
    pub fn get_user_config_dir_from(home: &Path) -> PathBuf {
        home.join(USER_CONFIG_DIR_RELATIVE)
    }

    /// Build the user config file path from an explicit home path.
    pub fn get_user_config_file_from(home: &Path) -> Option<PathBuf> {
        let config_path = Self::get_user_config_dir_from(home).join(USER_CONFIG_FILE_NAME);
        if config_path.exists() {
            Some(config_path)
        } else {
            None
        }
    }

    /// Find config file path in hierarchical order (local first, then user).
    pub fn get_config_file_from(start: &Path, home: &Path) -> Option<PathBuf> {
        Self::get_local_config_file_from(start).or_else(|| Self::get_user_config_file_from(home))
    }

    /// Load config and return it along with the local config directory.
    pub fn load_with_dir_from(
        start: &Path,
        home: &Path,
    ) -> Result<(Option<Config>, Option<PathBuf>)> {
        let user_config = Self::get_user_config_file_from(home);
        let local_config_dir = Self::get_local_config_dir_from(start);
        let local_config = local_config_dir
            .as_ref()
            .map(|dir| dir.join(LOCAL_CONFIG_FILE_NAME));

        let config = match (user_config, local_config) {
            (Some(user_path), Some(local_path)) => {
                let user = Config::from_file(&user_path)?;
                let local = Config::from_file(&local_path)?;
                Some(Config::merge(user, local))
            }
            (Some(user_path), None) => Some(Config::from_file(&user_path)?),
            (None, Some(local_path)) => Some(Config::from_file(&local_path)?),
            (None, None) => None,
        };

        Ok((config, local_config_dir))
    }

    /// Load config from explicit paths.
    pub fn load_from(start: &Path, home: &Path) -> Result<Option<Config>> {
        Ok(Self::load_with_dir_from(start, home)?.0)
    }

    /// Find config file path (local first, then user) from the current directory and $HOME.
    pub fn get_config_file() -> Result<Option<PathBuf>> {
        let current_dir = env::current_dir().context("Failed to get current directory")?;
        let home = shellexpand::tilde("~");
        Ok(Self::get_config_file_from(
            &current_dir,
            Path::new(home.as_ref()),
        ))
    }

    /// Load config from the current directory and $HOME.
    pub fn load() -> Result<Option<Config>> {
        Ok(Self::load_with_dir()?.0)
    }

    /// Load config and local config directory from the current directory and $HOME.
    pub fn load_with_dir() -> Result<(Option<Config>, Option<PathBuf>)> {
        let current_dir = env::current_dir().context("Failed to get current directory")?;
        let home = shellexpand::tilde("~");
        Self::load_with_dir_from(&current_dir, Path::new(home.as_ref()))
    }
}
