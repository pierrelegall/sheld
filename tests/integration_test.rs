// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

use indoc::indoc;
use shwrap::config::EntryType;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_full_config_loading_and_execution() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".shwrap");

    let yaml = indoc! {"
        base:
          type: model
          share:
            - user
          ro_bind:
            - /usr
            - /lib

        node:
          extends: base
          enabled: true
          bind:
            - ~/.npm:~/.npm
          env:
            NODE_ENV: production

        python:
          enabled: false
    "};

    fs::write(&config_path, yaml).unwrap();

    // Load and verify config
    use shwrap::config::Config;
    let config = Config::from_file(&config_path).unwrap();

    // Verify node command
    let node_cmd = config.get_command("node").unwrap();
    assert!(node_cmd.enabled);
    assert_eq!(node_cmd.extends, Some("base".to_string()));

    // Verify merging with base
    let merged = config.merge_with_base(node_cmd);
    assert!(merged.share.contains(&"user".to_string()));
    assert!(merged.ro_bind.contains(&"/usr".to_string()));
    assert_eq!(merged.env.get("NODE_ENV"), Some(&"production".to_string()));

    // Verify python command is disabled
    let python_cmd = config.get_command("python").unwrap();
    assert!(!python_cmd.enabled);
}

#[test]
fn test_bwrap_builder_integration() {
    use shwrap::bwrap::WrappedCommandBuilder;
    use shwrap::config::Entry;
    use std::collections::HashMap;

    let mut config = Entry {
        entry_type: EntryType::Command,
        enabled: true,
        extends: None,
        share: vec![],
        bind: vec!["/tmp:/tmp".to_string()],
        ro_bind: vec!["/usr".to_string()],
        dev_bind: vec![],
        tmpfs: vec!["/var/tmp".to_string()],
        env: HashMap::new(),
        unset_env: vec![],
    };
    config.env.insert("TEST".to_string(), "value".to_string());

    let builder = WrappedCommandBuilder::new(config);
    let args = builder.build_args();

    // All namespaces unshared by default
    assert!(args.contains(&"--unshare-net".to_string()));
    assert!(args.contains(&"--unshare-pid".to_string()));
    assert!(args.contains(&"--bind".to_string()));
    assert!(args.contains(&"--ro-bind".to_string()));
    assert!(args.contains(&"--tmpfs".to_string()));
    assert!(args.contains(&"--setenv".to_string()));
    assert!(args.contains(&"TEST".to_string()));
    assert!(args.contains(&"value".to_string()));

    // Test show command
    let cmd_line = builder.show("echo", &["hello".to_string()]);
    assert!(cmd_line.starts_with("bwrap"));
    assert!(cmd_line.contains("echo"));
    assert!(cmd_line.contains("hello"));
}

#[test]
fn test_config_with_all_features() {
    use shwrap::config::Config;
    let config = Config::from_yaml(indoc! {"
        base:
          type: model
          share:
            - user
          ro_bind:
            - /usr
            - /lib
          bind:
            - /src:/dest

        test:
          extends: base
          enabled: true
          dev_bind:
            - /dev/null
          tmpfs:
            - /tmp
          env:
            VAR1: value1
            VAR2: value2
          unset_env:
            - DEBUG
            - VERBOSE
    "})
    .unwrap();

    let test_cmd = config.get_command("test").unwrap();
    let merged = config.merge_with_base(test_cmd);

    // Verify all fields are populated correctly
    assert!(merged.enabled);
    assert_eq!(merged.share.len(), 1);
    assert_eq!(merged.ro_bind.len(), 2);
    assert_eq!(merged.bind.len(), 1);
    assert_eq!(merged.dev_bind.len(), 1);
    assert_eq!(merged.tmpfs.len(), 1);
    assert_eq!(merged.env.len(), 2);
    assert_eq!(merged.unset_env.len(), 2);

    // Build and verify bwrap args
    use shwrap::bwrap::WrappedCommandBuilder;
    let builder = WrappedCommandBuilder::new(merged);
    let args = builder.build_args();

    // User is shared, so no --unshare-user
    assert!(!args.contains(&"--unshare-user".to_string()));
    // But other namespaces should be unshared
    assert!(args.contains(&"--unshare-net".to_string()));
    assert!(args.contains(&"--unshare-pid".to_string()));
    assert!(args.contains(&"--dev-bind".to_string()));
    assert!(args.contains(&"--tmpfs".to_string()));
    assert!(args.contains(&"--setenv".to_string()));
    assert!(args.contains(&"--unsetenv".to_string()));
}

#[test]
fn test_multiple_commands_in_config() {
    use shwrap::config::Config;
    let config = Config::from_yaml(indoc! {"
        node:
          enabled: true
          share:
            - user
            - network
        python:
          enabled: true
          share:
            - user
        ruby:
          enabled: false
    "})
    .unwrap();

    let commands = config.get_commands();
    assert_eq!(commands.len(), 3);

    // Test each command
    let node = config.get_command("node").unwrap();
    assert!(node.enabled);
    assert_eq!(node.share, vec!["user", "network"]);

    let python = config.get_command("python").unwrap();
    assert!(python.enabled);
    assert_eq!(python.share, vec!["user"]);

    let ruby = config.get_command("ruby").unwrap();
    assert!(!ruby.enabled);
}

#[test]
fn test_config_error_handling() {
    use shwrap::config::Config;

    // Invalid YAML should error
    let result = Config::from_yaml(indoc! {"
        node
          this is not valid yaml
    "});
    assert!(result.is_err());

    // Non-existent file should error
    let result = Config::from_file("/nonexistent/path/.shwrap");
    assert!(result.is_err());
}

#[test]
fn test_command_show_formatting() {
    use shwrap::bwrap::WrappedCommandBuilder;
    use shwrap::config::Entry;
    use std::collections::HashMap;

    let config = Entry {
        entry_type: EntryType::Command,
        enabled: true,
        extends: None,
        share: vec![],
        bind: vec![],
        ro_bind: vec!["/usr".to_string()],
        dev_bind: vec![],
        tmpfs: vec![],
        env: HashMap::new(),
        unset_env: vec![],
    };

    let builder = WrappedCommandBuilder::new(config);
    let cmd = builder.show("ls", &["-la".to_string(), "/tmp".to_string()]);

    // Verify command format
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    assert_eq!(parts[0], "bwrap");
    assert!(parts.contains(&"--unshare-net"));
    assert!(parts.contains(&"--ro-bind"));
    assert!(parts.contains(&"/usr"));
    assert!(parts.contains(&"ls"));
    assert!(parts.contains(&"-la"));
    assert!(parts.contains(&"/tmp"));
}

#[test]
fn test_empty_commands_section() {
    use shwrap::config::Config;
    let config = Config::from_yaml("").unwrap();

    let commands = config.get_commands();
    assert_eq!(commands.len(), 0);
    assert!(config.get_command("any").is_none());
}

#[test]
fn test_base_without_commands() {
    use shwrap::config::Config;
    let config = Config::from_yaml(indoc! {"
        base:
          type: model
          share:
            - user
    "})
    .unwrap();

    let commands = config.get_commands();
    assert_eq!(commands.len(), 0);
}

#[test]
fn test_custom_template_name() {
    use shwrap::config::Config;
    let config = Config::from_yaml(indoc! {"
        minimal:
          type: model
          share:
            - user
            - network

        strict:
          type: model
          share:
            - user
          ro_bind:
            - /usr

        node:
          extends: minimal
          bind:
            - ~/.npm:~/.npm

        python:
          extends: strict
    "})
    .unwrap();

    // Test node with minimal template
    let node = config.get_command("node").unwrap();
    let merged_node = config.merge_with_template(node);
    assert_eq!(merged_node.share, vec!["user", "network"]);
    assert_eq!(merged_node.bind, vec!["~/.npm:~/.npm"]);

    // Test python with strict template
    let python = config.get_command("python").unwrap();
    let merged_python = config.merge_with_template(python);
    assert_eq!(merged_python.share, vec!["user"]);
    assert_eq!(merged_python.ro_bind, vec!["/usr"]);
}

#[test]
fn test_unshare_all_by_default_integration() {
    use shwrap::bwrap::WrappedCommandBuilder;
    use shwrap::config::Config;

    // Test 1: Empty config should unshare all namespaces
    let config = Config::from_yaml(indoc! {"
        isolated:
          enabled: true
          ro_bind:
            - /usr
    "})
    .unwrap();

    let isolated_cmd = config.get_command("isolated").unwrap();
    let builder = WrappedCommandBuilder::new(isolated_cmd);
    let cmd_line = builder.show("echo", &["test".to_string()]);

    // All namespaces should be unshared
    assert!(cmd_line.contains("--unshare-user"));
    assert!(cmd_line.contains("--unshare-pid"));
    assert!(cmd_line.contains("--unshare-net"));
    assert!(cmd_line.contains("--unshare-ipc"));
    assert!(cmd_line.contains("--unshare-uts"));
    assert!(cmd_line.contains("--unshare-cgroup"));
}

#[test]
fn test_share_specific_namespaces_integration() {
    use shwrap::bwrap::WrappedCommandBuilder;
    use shwrap::config::Config;

    // Test 2: Share only user and network namespaces
    let config = Config::from_yaml(indoc! {"
        network_enabled:
          enabled: true
          share:
            - user
            - network
          ro_bind:
            - /usr
    "})
    .unwrap();

    let network_cmd = config.get_command("network_enabled").unwrap();
    let builder = WrappedCommandBuilder::new(network_cmd);
    let cmd_line = builder.show("echo", &["test".to_string()]);

    // User and network should NOT be unshared
    assert!(!cmd_line.contains("--unshare-user"));
    assert!(!cmd_line.contains("--unshare-net"));

    // Other namespaces should still be unshared
    assert!(cmd_line.contains("--unshare-pid"));
    assert!(cmd_line.contains("--unshare-ipc"));
    assert!(cmd_line.contains("--unshare-uts"));
    assert!(cmd_line.contains("--unshare-cgroup"));
}

#[test]
fn test_share_multiple_namespaces_integration() {
    use shwrap::bwrap::WrappedCommandBuilder;
    use shwrap::config::Config;

    // Test 3: Share user, network, and ipc namespaces
    let config = Config::from_yaml(indoc! {"
        relaxed:
          enabled: true
          share:
            - user
            - network
            - ipc
          ro_bind:
            - /usr
    "})
    .unwrap();

    let relaxed_cmd = config.get_command("relaxed").unwrap();
    let builder = WrappedCommandBuilder::new(relaxed_cmd);
    let cmd_line = builder.show("echo", &["test".to_string()]);

    // User, network, and ipc should NOT be unshared
    assert!(!cmd_line.contains("--unshare-user"));
    assert!(!cmd_line.contains("--unshare-net"));
    assert!(!cmd_line.contains("--unshare-ipc"));

    // Remaining namespaces should be unshared
    assert!(cmd_line.contains("--unshare-pid"));
    assert!(cmd_line.contains("--unshare-uts"));
    assert!(cmd_line.contains("--unshare-cgroup"));
}

#[test]
fn test_share_all_namespaces_integration() {
    use shwrap::bwrap::WrappedCommandBuilder;
    use shwrap::config::Config;

    // Test 4: Share all namespaces (no isolation)
    let config = Config::from_yaml(indoc! {"
        no_isolation:
          enabled: true
          share:
            - user
            - pid
            - network
            - ipc
            - uts
            - cgroup
          ro_bind:
            - /usr
    "})
    .unwrap();

    let no_isolation_cmd = config.get_command("no_isolation").unwrap();
    let builder = WrappedCommandBuilder::new(no_isolation_cmd);
    let cmd_line = builder.show("echo", &["test".to_string()]);

    // No namespaces should be unshared
    assert!(!cmd_line.contains("--unshare-user"));
    assert!(!cmd_line.contains("--unshare-pid"));
    assert!(!cmd_line.contains("--unshare-net"));
    assert!(!cmd_line.contains("--unshare-ipc"));
    assert!(!cmd_line.contains("--unshare-uts"));
    assert!(!cmd_line.contains("--unshare-cgroup"));
}

#[test]
fn test_template_with_share_inheritance() {
    use shwrap::bwrap::WrappedCommandBuilder;
    use shwrap::config::Config;

    // Test 5: Template inheritance with share
    let config = Config::from_yaml(indoc! {"
        base:
          type: model
          share:
            - user
          ro_bind:
            - /usr
            - /lib

        app:
          extends: base
          share:
            - network
    "})
    .unwrap();

    let app_cmd = config.get_command("app").unwrap();
    let merged = config.merge_with_template(app_cmd);
    let builder = WrappedCommandBuilder::new(merged);
    let cmd_line = builder.show("echo", &["test".to_string()]);

    // User and network should NOT be unshared (inherited + added)
    assert!(!cmd_line.contains("--unshare-user"));
    assert!(!cmd_line.contains("--unshare-net"));

    // Other namespaces should be unshared
    assert!(cmd_line.contains("--unshare-pid"));
    assert!(cmd_line.contains("--unshare-ipc"));
    assert!(cmd_line.contains("--unshare-uts"));
    assert!(cmd_line.contains("--unshare-cgroup"));
}
