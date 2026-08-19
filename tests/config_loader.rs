// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

use indoc::indoc;
use sheld::config::loader::ConfigLoader;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_get_local_config_file_in_current_dir() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(ConfigLoader::local_config_name());
    fs::write(&config_path, "commands: {}").unwrap();

    let found = ConfigLoader::get_local_config_file_from(temp_dir.path());
    assert_eq!(found, Some(config_path));
}

#[test]
fn test_get_local_config_file_in_parent_dir() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(ConfigLoader::local_config_name());
    fs::write(&config_path, "commands: {}").unwrap();

    let sub_dir = temp_dir.path().join("subdir");
    fs::create_dir(&sub_dir).unwrap();

    let found = ConfigLoader::get_local_config_file_from(&sub_dir);
    assert_eq!(found, Some(config_path));
}

#[test]
fn test_get_local_config_file_not_found() {
    let temp_dir = TempDir::new().unwrap();

    let found = ConfigLoader::get_local_config_file_from(temp_dir.path());
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
fn test_load_without_config() {
    let temp_dir = TempDir::new().unwrap();

    let config = ConfigLoader::load_from(temp_dir.path(), temp_dir.path()).unwrap();
    assert!(config.is_none());
}

#[test]
fn test_get_config_file_hierarchy_local_first() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(ConfigLoader::local_config_name());
    fs::write(&config_path, "commands: {}").unwrap();

    let fake_home = TempDir::new().unwrap();

    let found = ConfigLoader::get_config_file_from(temp_dir.path(), fake_home.path());
    assert_eq!(found, Some(config_path));
}

#[test]
fn test_get_config_file_walks_up_directories() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(ConfigLoader::local_config_name());
    fs::write(&config_path, "commands: {}").unwrap();

    let sub2 = temp_dir.path().join("level1").join("level2");
    fs::create_dir_all(&sub2).unwrap();

    let found = ConfigLoader::get_local_config_file_from(&sub2);
    assert_eq!(found, Some(config_path));
}
