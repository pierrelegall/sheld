// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

//! Helpers shared by integration tests.

#![allow(dead_code)]

use sheld::config::loader::ConfigLoader;
use tempfile::TempDir;

/// YAML that fails to parse (unclosed flow sequence).
pub const INVALID_YAML: &str = "node:\n  enabled: true\n  share: [user\n";

/// Create a temporary directory that has no local config in any parent.
///
/// Config discovery walks up to `/`, so a stray `.sheld.yaml` in a parent of
/// the temp root would silently change test results. Fail loudly instead.
pub fn isolated_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    if let Some(stray) = ConfigLoader::get_local_config_file_from(dir.path()) {
        panic!(
            "{} is picked up by config discovery from {}; remove it to run these tests",
            stray.display(),
            dir.path().display()
        );
    }
    dir
}
