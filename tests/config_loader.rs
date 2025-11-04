// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

use indoc::indoc;
use shwrap::config::loader::ConfigLoader;
use std::env;
use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

// Mutex to ensure tests that change directory don't run in parallel
static DIR_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_find_local_config_in_current_dir() {
    let _lock = DIR_MUTEX.lock().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".shwrap");

    fs::write(&config_path, "commands: {}").unwrap();

    // Change to temp directory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&temp_dir).unwrap();

    let found = ConfigLoader::find_local_config().unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap(), config_path);

    // Restore original directory
    env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_find_local_config_in_parent_dir() {
    let _lock = DIR_MUTEX.lock().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".shwrap");
    fs::write(&config_path, "commands: {}").unwrap();

    // Create subdirectory
    let sub_dir = temp_dir.path().join("subdir");
    fs::create_dir(&sub_dir).unwrap();

    // Change to subdirectory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&sub_dir).unwrap();

    let found = ConfigLoader::find_local_config().unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap(), config_path);

    // Restore original directory
    env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_find_local_config_not_found() {
    let _lock = DIR_MUTEX.lock().unwrap();

    let temp_dir = TempDir::new().unwrap();

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&temp_dir).unwrap();

    let found = ConfigLoader::find_local_config().unwrap();
    assert!(found.is_none());

    env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_find_user_config() {
    // This test checks the logic without actually creating files in HOME
    // We can't easily test this without mocking HOME env var
    let result = ConfigLoader::find_user_config();
    assert!(result.is_ok());
}

#[test]
fn test_load_with_valid_config() {
    let _lock = DIR_MUTEX.lock().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".shwrap");

    let yaml = indoc! {"
        node:
          enabled: true
    "};
    fs::write(&config_path, yaml).unwrap();

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&temp_dir).unwrap();

    let config = ConfigLoader::load().unwrap();
    assert!(config.is_some());

    let config = config.unwrap();
    let commands = config.get_commands();
    assert_eq!(commands.len(), 1);
    assert!(commands.contains_key("node"));

    env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_load_without_config() {
    let _lock = DIR_MUTEX.lock().unwrap();

    let temp_dir = TempDir::new().unwrap();

    let original_dir = env::current_dir().unwrap();
    let original_home = env::var("HOME").ok();

    // Set HOME to temp dir to avoid finding user config
    env::set_var("HOME", temp_dir.path());
    env::set_current_dir(&temp_dir).unwrap();

    let config = ConfigLoader::load().unwrap();
    assert!(config.is_none());

    env::set_current_dir(original_dir).unwrap();

    // Restore original HOME
    if let Some(home) = original_home {
        env::set_var("HOME", home);
    } else {
        env::remove_var("HOME");
    }
}

#[test]
fn test_find_config_hierarchy_local_first() {
    let _lock = DIR_MUTEX.lock().unwrap();

    // Local config should take precedence over user/system configs
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".shwrap");
    fs::write(&config_path, "commands: {}").unwrap();

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&temp_dir).unwrap();

    let found = ConfigLoader::find_config().unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap(), config_path);

    env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_find_config_walks_up_directories() {
    let _lock = DIR_MUTEX.lock().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".shwrap");
    fs::write(&config_path, "commands: {}").unwrap();

    // Create nested subdirectories
    let sub1 = temp_dir.path().join("level1");
    let sub2 = sub1.join("level2");
    fs::create_dir_all(&sub2).unwrap();

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&sub2).unwrap();

    // Should find config in ancestor directory
    let found = ConfigLoader::find_local_config().unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap(), config_path);

    env::set_current_dir(original_dir).unwrap();
}
