// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

mod common;

use common::{INVALID_YAML, isolated_dir};

const WARNING: &str =
    "[sheld] WARNING: sheld will NOT sandbox any commands until this is fixed.\n\n";

/// Run `sheld <args>` with isolated working and home directories.
fn sheld(cwd: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sheld"))
        .args(args)
        .current_dir(cwd)
        .env("HOME", home)
        .output()
        .unwrap()
}

/// Run `sheld list`, optionally with `--simple`.
fn sheld_list(cwd: &Path, home: &Path, simple: bool) -> Output {
    let args: &[&str] = if simple {
        &["list", "--simple"]
    } else {
        &["list"]
    };
    sheld(cwd, home, args)
}

fn assert_invalid_config_warning(output: &Output, config_path: &Path) {
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1), "stderr:\n{stderr}");
    assert!(output.stdout.is_empty());
    assert!(
        stderr.starts_with(WARNING),
        "warning should come first, got:\n{stderr}"
    );
    assert!(stderr.contains("did not find expected ',' or ']'"));
    assert!(stderr.contains(&format!("[sheld] Please fix {}", config_path.display())));
}

#[test]
fn test_list_invalid_local_config_warns_and_fails() {
    let home = isolated_dir();
    let project = isolated_dir();
    let config_path = project.path().join(".sheld.yaml");
    fs::write(&config_path, INVALID_YAML).unwrap();

    for simple in [true, false] {
        let output = sheld_list(project.path(), home.path(), simple);
        assert_invalid_config_warning(&output, &config_path);
    }
}

#[test]
fn test_config_commands_warn_on_invalid_config() {
    let home = isolated_dir();
    let project = isolated_dir();
    let config_path = project.path().join(".sheld.yaml");
    fs::write(&config_path, INVALID_YAML).unwrap();

    for args in [
        &["wrap", "node"][..],
        &["show", "node"],
        &["check", "node"],
        &["check", "node", "--silent"],
    ] {
        let output = sheld(project.path(), home.path(), args);
        assert_invalid_config_warning(&output, &config_path);
    }
}

#[test]
fn test_check_reports_missing_command_without_warning() {
    let home = isolated_dir();
    let project = isolated_dir();
    fs::write(
        project.path().join(".sheld.yaml"),
        "node:\n  enabled: true\n",
    )
    .unwrap();

    let found = sheld(project.path(), home.path(), &["check", "node"]);
    assert!(found.status.success());

    let missing = sheld(project.path(), home.path(), &["check", "python"]);
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(stderr, "Command `python` not found in configuration\n");
}

#[test]
fn test_list_invalid_user_config_names_user_file() {
    let home = isolated_dir();
    let config_dir = home.path().join(".config/sheld");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("default.yaml");
    fs::write(&config_path, INVALID_YAML).unwrap();
    let project = isolated_dir();

    let output = sheld_list(project.path(), home.path(), true);
    assert_invalid_config_warning(&output, &config_path);
}

#[test]
fn test_list_simple_without_config_is_silent() {
    let home = isolated_dir();
    let project = isolated_dir();

    let output = sheld_list(project.path(), home.path(), true);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn test_list_without_config_reports_error() {
    let home = isolated_dir();
    let project = isolated_dir();

    let output = sheld_list(project.path(), home.path(), false);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("No configuration found"));
}

#[test]
fn test_list_simple_valid_config_prints_enabled_commands() {
    let home = isolated_dir();
    let project = isolated_dir();
    fs::write(
        project.path().join(".sheld.yaml"),
        "node:\n  enabled: true\npython:\n  enabled: false\n",
    )
    .unwrap();

    let output = sheld_list(project.path(), home.path(), true);

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "node\n");
    assert!(output.stderr.is_empty());
}
