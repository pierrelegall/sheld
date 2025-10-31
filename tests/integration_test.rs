use std::fs;
use tempfile::TempDir;

#[test]
fn test_full_config_loading_and_execution() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".bwrap");

    let yaml = r#"
templates:
  base:
    unshare:
      - network
    ro_bind:
      - /usr
      - /lib

commands:
  node:
    extends: base
    enabled: true
    bind:
      - ~/.npm:~/.npm
    env:
      NODE_ENV: production

  python:
    enabled: false
    unshare:
      - pid
"#;

    fs::write(&config_path, yaml).unwrap();

    // Load and verify config
    use bwrap_manager::config::BwrapConfig;
    let config = BwrapConfig::from_file(&config_path).unwrap();

    // Verify templates/base config
    assert_eq!(config.templates.len(), 1);
    assert!(config.templates.contains_key("base"));
    let base = config.templates.get("base").unwrap();
    assert_eq!(base.unshare, vec!["network"]);
    assert_eq!(base.ro_bind.len(), 2);

    // Verify node command
    let node_cmd = config.get_command_config("node").unwrap();
    assert!(node_cmd.enabled);
    assert_eq!(node_cmd.extends, Some("base".to_string()));

    // Verify merging with base
    let merged = config.merge_with_base(node_cmd);
    assert!(merged.unshare.contains(&"network".to_string()));
    assert!(merged.ro_bind.contains(&"/usr".to_string()));
    assert_eq!(merged.env.get("NODE_ENV"), Some(&"production".to_string()));

    // Verify python command is disabled
    let python_cmd = config.get_command_config("python").unwrap();
    assert!(!python_cmd.enabled);
}

#[test]
fn test_bwrap_builder_integration() {
    use bwrap_manager::config::CommandConfig;
    use bwrap_manager::bwrap::BwrapBuilder;
    use std::collections::HashMap;

    let mut config = CommandConfig {
        enabled: true,
        extends: None,
        unshare: vec!["network".to_string(), "pid".to_string()],
        share: vec![],
        bind: vec!["/tmp:/tmp".to_string()],
        ro_bind: vec!["/usr".to_string()],
        dev_bind: vec![],
        tmpfs: vec!["/var/tmp".to_string()],
        env: HashMap::new(),
        unset_env: vec![],
    };
    config.env.insert("TEST".to_string(), "value".to_string());

    let builder = BwrapBuilder::new(config);
    let args = builder.build_args();

    // Verify all arguments are present
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
    let yaml = r#"
templates:
  base:
    unshare:
      - network
      - pid
    share:
      - /home/user
    ro_bind:
      - /usr
      - /lib
    bind:
      - /src:/dest

commands:
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
"#;

    use bwrap_manager::config::BwrapConfig;
    let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();

    let test_cmd = config.get_command_config("test").unwrap();
    let merged = config.merge_with_base(test_cmd);

    // Verify all fields are populated correctly
    assert!(merged.enabled);
    assert_eq!(merged.unshare.len(), 2);
    assert_eq!(merged.share.len(), 1);
    assert_eq!(merged.ro_bind.len(), 2);
    assert_eq!(merged.bind.len(), 1);
    assert_eq!(merged.dev_bind.len(), 1);
    assert_eq!(merged.tmpfs.len(), 1);
    assert_eq!(merged.env.len(), 2);
    assert_eq!(merged.unset_env.len(), 2);

    // Build and verify bwrap args
    use bwrap_manager::bwrap::BwrapBuilder;
    let builder = BwrapBuilder::new(merged);
    let args = builder.build_args();

    // Should contain all types of arguments
    assert!(args.contains(&"--unshare-net".to_string()));
    assert!(args.contains(&"--unshare-pid".to_string()));
    assert!(args.contains(&"--dev-bind".to_string()));
    assert!(args.contains(&"--tmpfs".to_string()));
    assert!(args.contains(&"--setenv".to_string()));
    assert!(args.contains(&"--unsetenv".to_string()));
}

#[test]
fn test_multiple_commands_in_config() {
    let yaml = r#"
commands:
  node:
    enabled: true
    unshare:
      - network
  python:
    enabled: true
    unshare:
      - pid
  ruby:
    enabled: false
"#;

    use bwrap_manager::config::BwrapConfig;
    let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();

    assert_eq!(config.commands.len(), 3);

    // Test each command
    let node = config.get_command_config("node").unwrap();
    assert!(node.enabled);
    assert_eq!(node.unshare, vec!["network"]);

    let python = config.get_command_config("python").unwrap();
    assert!(python.enabled);
    assert_eq!(python.unshare, vec!["pid"]);

    let ruby = config.get_command_config("ruby").unwrap();
    assert!(!ruby.enabled);
}

#[test]
fn test_config_error_handling() {
    use bwrap_manager::config::BwrapConfig;

    // Invalid YAML should error
    let invalid_yaml = r#"
commands:
  node
    this is not valid yaml
"#;

    let result: Result<BwrapConfig, _> = serde_yaml::from_str(invalid_yaml);
    assert!(result.is_err());

    // Non-existent file should error
    let result = BwrapConfig::from_file("/nonexistent/path/.bwrap");
    assert!(result.is_err());
}

#[test]
fn test_command_show_formatting() {
    use bwrap_manager::config::CommandConfig;
    use bwrap_manager::bwrap::BwrapBuilder;
    use std::collections::HashMap;

    let config = CommandConfig {
        enabled: true,
        extends: None,
        unshare: vec!["network".to_string()],
        share: vec![],
        bind: vec![],
        ro_bind: vec!["/usr".to_string()],
        dev_bind: vec![],
        tmpfs: vec![],
        env: HashMap::new(),
        unset_env: vec![],
    };

    let builder = BwrapBuilder::new(config);
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
    let yaml = r#"
commands: {}
"#;

    use bwrap_manager::config::BwrapConfig;
    let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();

    assert_eq!(config.commands.len(), 0);
    assert!(config.get_command_config("any").is_none());
}

#[test]
fn test_base_without_commands() {
    let yaml = r#"
templates:
  base:
    unshare:
      - network
"#;

    use bwrap_manager::config::BwrapConfig;
    let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();

    assert_eq!(config.templates.len(), 1);
    assert!(config.templates.contains_key("base"));
    assert_eq!(config.commands.len(), 0);
}

#[test]
fn test_custom_template_name() {
    let yaml = r#"
templates:
  minimal:
    unshare:
      - network
  strict:
    unshare:
      - network
      - pid

commands:
  node:
    extends: minimal
    bind:
      - ~/.npm:~/.npm
  python:
    extends: strict
"#;

    use bwrap_manager::config::BwrapConfig;
    let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();

    // Verify templates
    assert_eq!(config.templates.len(), 2);
    assert!(config.templates.contains_key("minimal"));
    assert!(config.templates.contains_key("strict"));

    // Test node with minimal template
    let node = config.get_command_config("node").unwrap();
    let merged_node = config.merge_with_template(node);
    assert_eq!(merged_node.unshare, vec!["network"]);
    assert_eq!(merged_node.bind, vec!["~/.npm:~/.npm"]);

    // Test python with strict template
    let python = config.get_command_config("python").unwrap();
    let merged_python = config.merge_with_template(python);
    assert_eq!(merged_python.unshare, vec!["network", "pid"]);
}
