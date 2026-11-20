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
    sheld_with_env(cwd, home, args, &[])
}

/// Run `sheld <args>` like [`sheld`], with extra environment variables.
fn sheld_with_env(cwd: &Path, home: &Path, args: &[&str], envs: &[(&str, &str)]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sheld"))
        .args(args)
        .current_dir(cwd)
        .env("HOME", home)
        .envs(envs.iter().copied())
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

/// Cause reported for `INVALID_YAML`.
const PARSE_CAUSE: &str = "did not find expected ',' or ']'";

/// Assert the invalid configuration warning for `config_path`, reporting `cause`.
fn assert_invalid_config_warning(output: &Output, config_path: &Path, cause: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1), "stderr:\n{stderr}");
    assert!(output.stdout.is_empty());
    assert!(
        stderr.starts_with(WARNING),
        "warning should come first, got:\n{stderr}"
    );
    assert!(
        stderr.contains(&format!("[sheld]   {cause}")),
        "cause {cause:?} missing, got:\n{stderr}"
    );
    assert!(stderr.contains(&format!("[sheld] Please fix {}", config_path.display())));
}

#[test]
fn test_list_invalid_local_config_warns_and_fails() {
    let home = isolated_dir();
    let project = isolated_dir();
    let config_path = project.path().join("sheld.yaml");
    fs::write(&config_path, INVALID_YAML).unwrap();

    for simple in [true, false] {
        let output = sheld_list(project.path(), home.path(), simple);
        assert_invalid_config_warning(&output, &config_path, PARSE_CAUSE);
        assert!(String::from_utf8_lossy(&output.stderr).contains(&format!(
            "[sheld] (run `sheld validate {}` to check it)",
            config_path.display()
        )));
    }
}

#[test]
fn test_config_commands_warn_on_invalid_config() {
    let home = isolated_dir();
    let project = isolated_dir();
    let config_path = project.path().join("sheld.yaml");
    fs::write(&config_path, INVALID_YAML).unwrap();

    for args in [
        &["wrap", "node"][..],
        &["show", "node"],
        &["check", "node"],
        &["check", "node", "--silent"],
    ] {
        let output = sheld(project.path(), home.path(), args);
        assert_invalid_config_warning(&output, &config_path, PARSE_CAUSE);
    }
}

#[test]
fn test_check_reports_missing_command_without_warning() {
    let home = isolated_dir();
    let project = isolated_dir();
    fs::write(
        project.path().join("sheld.yaml"),
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
    assert_invalid_config_warning(&output, &config_path, PARSE_CAUSE);
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
        project.path().join("sheld.yaml"),
        "node:\n  enabled: true\npython:\n  enabled: false\n",
    )
    .unwrap();

    let output = sheld_list(project.path(), home.path(), true);

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "node\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn test_unset_path_variable_warning_names_variable() {
    let home = isolated_dir();
    let project = isolated_dir();
    let config_path = project.path().join("sheld.yaml");
    fs::write(
        &config_path,
        "node:\n  bind:\n    - $SHELD_CLI_VARIABLE_THAT_MUST_NOT_EXIST_7C21\n",
    )
    .unwrap();

    let output = sheld_list(project.path(), home.path(), true);
    assert_invalid_config_warning(
        &output,
        &config_path,
        "environment variable `SHELD_CLI_VARIABLE_THAT_MUST_NOT_EXIST_7C21` is not set \
         (used in bind path \"$SHELD_CLI_VARIABLE_THAT_MUST_NOT_EXIST_7C21\" of entry \"node\")",
    );
}

#[test]
fn test_empty_path_variable_warning_names_variable() {
    let home = isolated_dir();
    let project = isolated_dir();
    let config_path = project.path().join("sheld.yaml");
    fs::write(
        &config_path,
        "node:\n  bind:\n    - $SHELD_CLI_EMPTY_7C21/cache\n",
    )
    .unwrap();

    let output = sheld_with_env(
        project.path(),
        home.path(),
        &["list", "--simple"],
        &[("SHELD_CLI_EMPTY_7C21", "")],
    );
    assert_invalid_config_warning(
        &output,
        &config_path,
        "environment variable `SHELD_CLI_EMPTY_7C21` is empty \
         (used in bind path \"$SHELD_CLI_EMPTY_7C21/cache\" of entry \"node\")",
    );
}

#[test]
fn test_path_variable_fallback_used_when_empty() {
    let home = isolated_dir();
    let project = isolated_dir();
    fs::write(
        project.path().join("sheld.yaml"),
        "node:\n  bind:\n    - ${SHELD_CLI_EMPTY_7C21:-/fallback/cache}\n",
    )
    .unwrap();

    let output = sheld_with_env(
        project.path(),
        home.path(),
        &["show", "node"],
        &[("SHELD_CLI_EMPTY_7C21", "")],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "{output:?}");
    assert!(
        stdout.contains("--bind /fallback/cache /fallback/cache"),
        "got:\n{stdout}"
    );
}

#[test]
fn test_legacy_local_config_name_warns_to_rename() {
    for (with_new_name, cause) in [
        (
            false,
            "`.sheld.yaml` is no longer read: rename it to `sheld.yaml`",
        ),
        (
            true,
            "`.sheld.yaml` is no longer read: merge it into `sheld.yaml` and remove it",
        ),
    ] {
        let home = isolated_dir();
        let project = isolated_dir();
        if with_new_name {
            fs::write(
                project.path().join("sheld.yaml"),
                "node:\n  enabled: true\n",
            )
            .unwrap();
        }
        let legacy_path = project.path().join(".sheld.yaml");
        fs::write(&legacy_path, "node:\n  enabled: true\n").unwrap();

        let output = sheld_list(project.path(), home.path(), true);
        // The file is valid: `sheld validate` would not help.
        assert!(!String::from_utf8_lossy(&output.stderr).contains("sheld validate"));
        assert_invalid_config_warning(&output, &legacy_path, cause);
    }
}

#[test]
fn test_init_writes_sheld_yaml() {
    let home = isolated_dir();
    let project = isolated_dir();

    let output = sheld(project.path(), home.path(), &["init"]);

    assert!(output.status.success(), "{output:?}");
    assert!(project.path().join("sheld.yaml").is_file());
}

#[test]
fn test_init_refuses_when_sheld_yaml_exists() {
    let home = isolated_dir();
    let project = isolated_dir();
    fs::write(
        project.path().join("sheld.yaml"),
        "node:\n  enabled: true\n",
    )
    .unwrap();

    let output = sheld(project.path(), home.path(), &["init"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("sheld.yaml file already exists in current directory"),
        "got:\n{stderr}"
    );
}

#[test]
fn test_init_asks_to_rename_legacy_config() {
    let home = isolated_dir();
    let project = isolated_dir();
    let legacy_path = project.path().join(".sheld.yaml");
    fs::write(&legacy_path, "node:\n  enabled: true\n").unwrap();

    let output = sheld(project.path(), home.path(), &["init"]);

    assert_invalid_config_warning(
        &output,
        &legacy_path,
        "`.sheld.yaml` is no longer read: rename it to `sheld.yaml`",
    );
    assert!(!project.path().join("sheld.yaml").exists());
}
