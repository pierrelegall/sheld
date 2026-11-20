// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

use indoc::indoc;
use sheld::config::loader::{ConfigFileError, ConfigLoader};
use std::fs;
use tempfile::TempDir;

mod common;

use common::{INVALID_YAML, isolated_dir};

#[test]
fn test_get_local_config_file_in_current_dir() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(ConfigLoader::local_config_name());
    fs::write(&config_path, "commands: {}").unwrap();

    let found = ConfigLoader::get_local_config_file_from(temp_dir.path()).unwrap();
    assert_eq!(found, Some(config_path));
}

#[test]
fn test_get_local_config_file_in_parent_dir() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(ConfigLoader::local_config_name());
    fs::write(&config_path, "commands: {}").unwrap();

    let sub_dir = temp_dir.path().join("subdir");
    fs::create_dir(&sub_dir).unwrap();

    let found = ConfigLoader::get_local_config_file_from(&sub_dir).unwrap();
    assert_eq!(found, Some(config_path));
}

#[test]
fn test_get_local_config_file_not_found() {
    let temp_dir = isolated_dir();

    let found = ConfigLoader::get_local_config_file_from(temp_dir.path()).unwrap();
    assert!(found.is_none());
}

#[test]
fn test_get_user_config_file() {
    let fake_home = TempDir::new().unwrap();
    let config_dir = fake_home.path().join(".config").join("sheld");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join(ConfigLoader::user_config_name()),
        "commands: {}",
    )
    .unwrap();

    let found = ConfigLoader::get_user_config_file_from(fake_home.path());
    assert!(found.is_some());
}

#[test]
fn test_load_with_valid_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(ConfigLoader::local_config_name());

    let yaml = indoc! {"
        node:
          enabled: true
    "};
    fs::write(&config_path, yaml).unwrap();

    let config = ConfigLoader::load_from(temp_dir.path(), temp_dir.path()).unwrap();
    assert!(config.is_some());

    let config = config.unwrap();
    let commands = config.get_commands();
    assert_eq!(commands.len(), 1);
    assert!(commands.contains_key("node"));
}

#[test]
fn test_load_resolves_user_and_local_paths_against_their_own_files() {
    let fake_home = TempDir::new().unwrap();
    let user_config_dir = fake_home.path().join(".config").join("sheld");
    fs::create_dir_all(&user_config_dir).unwrap();
    fs::write(
        user_config_dir.join(ConfigLoader::user_config_name()),
        "node:\n  bind:\n    - ./user-src\n",
    )
    .unwrap();

    let local_dir = TempDir::new().unwrap();
    fs::write(
        local_dir.path().join(ConfigLoader::local_config_name()),
        "node:\n  bind:\n    - ./local-src\n",
    )
    .unwrap();

    let config = ConfigLoader::load_from(local_dir.path(), fake_home.path())
        .unwrap()
        .unwrap();
    let node = config.get_command("node").unwrap();
    let user_path = user_config_dir
        .join("user-src")
        .to_string_lossy()
        .into_owned();
    let local_path = local_dir
        .path()
        .join("local-src")
        .to_string_lossy()
        .into_owned();

    assert_eq!(node.bind.get(&user_path), Some(&user_path));
    assert_eq!(node.bind.get(&local_path), Some(&local_path));
}

#[test]
fn test_load_without_config() {
    let temp_dir = isolated_dir();

    let config = ConfigLoader::load_from(temp_dir.path(), temp_dir.path()).unwrap();
    assert!(config.is_none());
}

#[test]
fn test_get_config_file_hierarchy_local_first() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(ConfigLoader::local_config_name());
    fs::write(&config_path, "commands: {}").unwrap();

    let fake_home = TempDir::new().unwrap();

    let found = ConfigLoader::get_config_file_from(temp_dir.path(), fake_home.path()).unwrap();
    assert_eq!(found, Some(config_path));
}

#[test]
fn test_get_config_file_walks_up_directories() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(ConfigLoader::local_config_name());
    fs::write(&config_path, "commands: {}").unwrap();

    let sub2 = temp_dir.path().join("level1").join("level2");
    fs::create_dir_all(&sub2).unwrap();

    let found = ConfigLoader::get_local_config_file_from(&sub2).unwrap();
    assert_eq!(found, Some(config_path));
}

fn write_user_config(home: &std::path::Path, content: &str) -> std::path::PathBuf {
    let config_dir = ConfigLoader::get_user_config_dir_from(home);
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join(ConfigLoader::user_config_name());
    fs::write(&config_path, content).unwrap();
    config_path
}

#[test]
fn test_load_invalid_local_config_reports_its_path() {
    let fake_home = TempDir::new().unwrap();
    let local_dir = TempDir::new().unwrap();
    let local_path = local_dir.path().join(ConfigLoader::local_config_name());
    fs::write(&local_path, INVALID_YAML).unwrap();

    let err = ConfigLoader::load_from(local_dir.path(), fake_home.path()).unwrap_err();
    let file_err = err
        .downcast_ref::<ConfigFileError>()
        .expect("error should carry the failing file path");
    assert_eq!(file_err.path, local_path);
}

#[test]
fn test_load_invalid_user_config_reports_its_path() {
    let fake_home = TempDir::new().unwrap();
    let user_path = write_user_config(fake_home.path(), INVALID_YAML);

    let local_dir = TempDir::new().unwrap();
    fs::write(
        local_dir.path().join(ConfigLoader::local_config_name()),
        "node:\n  enabled: true\n",
    )
    .unwrap();

    let err = ConfigLoader::load_from(local_dir.path(), fake_home.path()).unwrap_err();
    let file_err = err.downcast_ref::<ConfigFileError>().unwrap();
    assert_eq!(file_err.path, user_path);
}

#[test]
fn test_load_invalid_local_config_with_valid_user_config_reports_local_path() {
    let fake_home = TempDir::new().unwrap();
    write_user_config(fake_home.path(), "node:\n  enabled: true\n");

    let local_dir = TempDir::new().unwrap();
    let local_path = local_dir.path().join(ConfigLoader::local_config_name());
    fs::write(&local_path, INVALID_YAML).unwrap();

    let err = ConfigLoader::load_from(local_dir.path(), fake_home.path()).unwrap_err();
    let file_err = err.downcast_ref::<ConfigFileError>().unwrap();
    assert_eq!(file_err.path, local_path);
}

fn legacy_error(dir: &std::path::Path, home: &std::path::Path) -> (std::path::PathBuf, String) {
    let err = ConfigLoader::load_from(dir, home).unwrap_err();
    let file_err = err.downcast_ref::<ConfigFileError>().unwrap();
    assert!(!file_err.validate_hint);
    (file_err.path.clone(), err.root_cause().to_string())
}

#[test]
fn test_legacy_local_config_name_is_rejected() {
    let fake_home = isolated_dir();
    let local_dir = isolated_dir();
    let legacy_path = local_dir
        .path()
        .join(ConfigLoader::legacy_local_config_name());
    fs::write(&legacy_path, "node:\n  enabled: true\n").unwrap();

    let (path, cause) = legacy_error(local_dir.path(), fake_home.path());
    assert_eq!(path, legacy_path);
    assert_eq!(
        cause,
        "`.sheld.yaml` is no longer read: rename it to `sheld.yaml`"
    );
}

#[test]
fn test_legacy_local_config_name_next_to_new_name_is_rejected() {
    let fake_home = isolated_dir();
    let local_dir = isolated_dir();
    fs::write(
        local_dir.path().join(ConfigLoader::local_config_name()),
        "node:\n  enabled: true\n",
    )
    .unwrap();
    let legacy_path = local_dir
        .path()
        .join(ConfigLoader::legacy_local_config_name());
    fs::write(&legacy_path, "node:\n  enabled: true\n").unwrap();

    let (path, cause) = legacy_error(local_dir.path(), fake_home.path());
    assert_eq!(path, legacy_path);
    assert_eq!(
        cause,
        "`.sheld.yaml` is no longer read: merge it into `sheld.yaml` and remove it"
    );
}

#[test]
fn test_legacy_local_config_name_is_not_skipped_for_a_parent_config() {
    let fake_home = isolated_dir();
    let parent = isolated_dir();
    fs::write(
        parent.path().join(ConfigLoader::local_config_name()),
        "node:\n  enabled: true\n",
    )
    .unwrap();
    let sub_dir = parent.path().join("sub");
    fs::create_dir(&sub_dir).unwrap();
    let legacy_path = sub_dir.join(ConfigLoader::legacy_local_config_name());
    fs::write(&legacy_path, "node:\n  enabled: true\n").unwrap();

    let (path, _) = legacy_error(&sub_dir, fake_home.path());
    assert_eq!(path, legacy_path);
}
