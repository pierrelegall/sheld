// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result, anyhow};
use std::env;
use std::path::{Path, PathBuf};

use super::Config;

/// Local config file name
const LOCAL_CONFIG_FILE_NAME: &str = "sheld.yaml";

/// Former local config file name: no longer read, only detected to ask for a rename
const LEGACY_LOCAL_CONFIG_FILE_NAME: &str = ".sheld.yaml";

/// User config file name
const USER_CONFIG_FILE_NAME: &str = "default.yaml";

/// User config directory path relative to HOME (for display)
const USER_CONFIG_DIR_PATH: &str = "~/.config/sheld";

/// User config subdirectory relative to home
const USER_CONFIG_DIR_RELATIVE: &str = ".config/sheld";

/// Error context attached when a configuration file cannot be read or parsed.
///
/// Retrieve it with `err.downcast_ref::<ConfigFileError>()` to know which file failed.
#[derive(Debug, Clone)]
pub struct ConfigFileError {
    pub path: PathBuf,
    /// Whether `sheld validate <path>` helps diagnose the problem: false when
    /// the file itself may be fine, e.g. a former `.sheld.yaml`.
    pub validate_hint: bool,
}

impl std::fmt::Display for ConfigFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid configuration file {}", self.path.display())
    }
}

impl std::error::Error for ConfigFileError {}

/// Load a single config file, tagging any error with the file path.
fn load_file(path: &Path) -> Result<Config> {
    Config::from_file(path).context(ConfigFileError {
        path: path.to_path_buf(),
        validate_hint: true,
    })
}

pub struct ConfigLoader;

impl ConfigLoader {
    // --- Constants ---

    /// Local config file name, written by `sheld init`.
    pub fn local_config_name() -> &'static str {
        LOCAL_CONFIG_FILE_NAME
    }

    /// Former local config file name, no longer read: finding it is an error.
    pub fn legacy_local_config_name() -> &'static str {
        LEGACY_LOCAL_CONFIG_FILE_NAME
    }

    pub fn user_config_name() -> &'static str {
        USER_CONFIG_FILE_NAME
    }

    pub fn user_config_dir() -> &'static str {
        USER_CONFIG_DIR_PATH
    }

    // --- Core logic (explicit paths, no global state) ---

    /// Find the local config file in `dir` itself, without looking at parents.
    ///
    /// Fails if the former `.sheld.yaml` is there: ignoring it would silently
    /// stop sandboxing the commands it configures.
    pub fn get_local_config_file_in(dir: &Path) -> Result<Option<PathBuf>> {
        let config = dir.join(LOCAL_CONFIG_FILE_NAME);
        let legacy = dir.join(LEGACY_LOCAL_CONFIG_FILE_NAME);
        if legacy.exists() {
            let fix = if config.exists() {
                format!("merge it into `{LOCAL_CONFIG_FILE_NAME}` and remove it")
            } else {
                format!("rename it to `{LOCAL_CONFIG_FILE_NAME}`")
            };
            return Err(anyhow!(
                "`{LEGACY_LOCAL_CONFIG_FILE_NAME}` is no longer read: {fix}"
            ))
            .context(ConfigFileError {
                path: legacy,
                validate_hint: false,
            });
        }
        Ok(config.exists().then_some(config))
    }

    /// Walk up from `start` to find the local config file.
    pub fn get_local_config_file_from(start: &Path) -> Result<Option<PathBuf>> {
        for dir in start.ancestors() {
            if let Some(path) = Self::get_local_config_file_in(dir)? {
                return Ok(Some(path));
            }
        }
        Ok(None)
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
    pub fn get_config_file_from(start: &Path, home: &Path) -> Result<Option<PathBuf>> {
        Ok(Self::get_local_config_file_from(start)?
            .or_else(|| Self::get_user_config_file_from(home)))
    }

    /// Load config and return it along with the local config directory.
    pub fn load_with_dir_from(
        start: &Path,
        home: &Path,
    ) -> Result<(Option<Config>, Option<PathBuf>)> {
        let user_config = Self::get_user_config_file_from(home);
        let local_config = Self::get_local_config_file_from(start)?;
        let local_config_dir = local_config
            .as_ref()
            .and_then(|path| path.parent().map(Path::to_path_buf));

        let config = match (user_config, local_config) {
            (Some(user_path), Some(local_path)) => {
                let user = load_file(&user_path)?;
                let local = load_file(&local_path)?;
                Some(Config::merge(user, local))
            }
            (Some(user_path), None) => Some(load_file(&user_path)?),
            (None, Some(local_path)) => Some(load_file(&local_path)?),
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
        Self::get_config_file_from(&current_dir, Path::new(home.as_ref()))
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
