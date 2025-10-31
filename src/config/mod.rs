use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub mod loader;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BwrapConfig {
    #[serde(default)]
    pub commands: HashMap<String, CommandConfig>,
    #[serde(default)]
    pub templates: HashMap<String, TemplateConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    #[serde(default)]
    pub unshare: Vec<String>,
    #[serde(default)]
    pub share: Vec<String>,
    #[serde(default)]
    pub bind: Vec<String>,
    #[serde(default)]
    pub ro_bind: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub extends: Option<String>,
    #[serde(default)]
    pub unshare: Vec<String>,
    #[serde(default)]
    pub share: Vec<String>,
    #[serde(default)]
    pub bind: Vec<String>,
    #[serde(default)]
    pub ro_bind: Vec<String>,
    #[serde(default)]
    pub dev_bind: Vec<String>,
    #[serde(default)]
    pub tmpfs: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub unset_env: Vec<String>,
}

fn default_enabled() -> bool {
    true
}

impl BwrapConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .context(format!("Failed to read config file: {:?}", path.as_ref()))?;

        let config: BwrapConfig = serde_yaml::from_str(&content)
            .context("Failed to parse YAML config")?;

        Ok(config)
    }

    pub fn get_command_config(&self, command: &str) -> Option<CommandConfig> {
        self.commands.get(command).cloned()
    }

    pub fn merge_with_template(&self, mut cmd_config: CommandConfig) -> CommandConfig {
        if let Some(extends) = &cmd_config.extends {
            if let Some(template) = self.templates.get(extends) {
                // Merge template config into command config
                cmd_config.unshare.extend(template.unshare.clone());
                cmd_config.share.extend(template.share.clone());
                cmd_config.bind.extend(template.bind.clone());
                cmd_config.ro_bind.extend(template.ro_bind.clone());
            }
        }
        cmd_config
    }

    // Deprecated: use merge_with_template instead
    pub fn merge_with_base(&self, cmd_config: CommandConfig) -> CommandConfig {
        self.merge_with_template(cmd_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_basic_config() {
        let yaml = r#"
commands:
  node:
    enabled: true
    unshare:
      - network
    bind:
      - ~/.npm:~/.npm
"#;
        let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.commands.len(), 1);
        assert!(config.commands.contains_key("node"));

        let node_cmd = config.commands.get("node").unwrap();
        assert!(node_cmd.enabled);
        assert_eq!(node_cmd.unshare, vec!["network"]);
        assert_eq!(node_cmd.bind, vec!["~/.npm:~/.npm"]);
    }

    #[test]
    fn test_parse_config_with_base() {
        let yaml = r#"
templates:
  base:
    unshare:
      - network
      - pid
    ro_bind:
      - /usr
      - /lib

commands:
  node:
    extends: base
    bind:
      - ~/.npm:~/.npm
"#;
        let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.templates.len(), 1);
        assert!(config.templates.contains_key("base"));

        let base = config.templates.get("base").unwrap();
        assert_eq!(base.unshare, vec!["network", "pid"]);
        assert_eq!(base.ro_bind, vec!["/usr", "/lib"]);

        let node_cmd = config.commands.get("node").unwrap();
        assert_eq!(node_cmd.extends, Some("base".to_string()));
    }

    #[test]
    fn test_get_command_config() {
        let yaml = r#"
commands:
  node:
    enabled: true
  python:
    enabled: false
"#;
        let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(config.get_command_config("node").is_some());
        assert!(config.get_command_config("python").is_some());
        assert!(config.get_command_config("ruby").is_none());
    }

    #[test]
    fn test_merge_with_base() {
        let yaml = r#"
templates:
  base:
    unshare:
      - network
    ro_bind:
      - /usr

commands:
  node:
    extends: base
    bind:
      - ~/.npm:~/.npm
"#;
        let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();
        let node_cmd = config.get_command_config("node").unwrap();
        let merged = config.merge_with_base(node_cmd);

        // Should have both base and command-specific settings
        assert_eq!(merged.unshare, vec!["network"]);
        assert_eq!(merged.ro_bind, vec!["/usr"]);
        assert_eq!(merged.bind, vec!["~/.npm:~/.npm"]);
    }

    #[test]
    fn test_merge_without_extends() {
        let yaml = r#"
templates:
  base:
    unshare:
      - network

commands:
  node:
    bind:
      - ~/.npm:~/.npm
"#;
        let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();
        let node_cmd = config.get_command_config("node").unwrap();
        let merged = config.merge_with_base(node_cmd.clone());

        // Should not merge base since extends is not set
        assert_eq!(merged.unshare, node_cmd.unshare);
        assert_eq!(merged.bind, node_cmd.bind);
    }

    #[test]
    fn test_from_file() {
        let yaml = r#"
commands:
  test:
    enabled: true
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();

        let config = BwrapConfig::from_file(temp_file.path()).unwrap();
        assert_eq!(config.commands.len(), 1);
        assert!(config.commands.contains_key("test"));
    }

    #[test]
    fn test_default_enabled() {
        let yaml = r#"
commands:
  node:
    unshare:
      - network
"#;
        let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();
        let node_cmd = config.get_command_config("node").unwrap();
        // enabled should default to true
        assert!(node_cmd.enabled);
    }

    #[test]
    fn test_disabled_command() {
        let yaml = r#"
commands:
  node:
    enabled: false
    unshare:
      - network
"#;
        let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();
        let node_cmd = config.get_command_config("node").unwrap();
        assert!(!node_cmd.enabled);
    }

    #[test]
    fn test_env_variables() {
        let yaml = r#"
commands:
  node:
    env:
      NODE_ENV: production
      PATH: /custom/path
    unset_env:
      - DEBUG
"#;
        let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();
        let node_cmd = config.get_command_config("node").unwrap();

        assert_eq!(node_cmd.env.len(), 2);
        assert_eq!(node_cmd.env.get("NODE_ENV"), Some(&"production".to_string()));
        assert_eq!(node_cmd.unset_env, vec!["DEBUG"]);
    }

    #[test]
    fn test_tmpfs() {
        let yaml = r#"
commands:
  node:
    tmpfs:
      - /tmp
      - /var/tmp
"#;
        let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();
        let node_cmd = config.get_command_config("node").unwrap();
        assert_eq!(node_cmd.tmpfs, vec!["/tmp", "/var/tmp"]);
    }

    #[test]
    fn test_dev_bind() {
        let yaml = r#"
commands:
  node:
    dev_bind:
      - /dev/null
      - /dev/random
"#;
        let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();
        let node_cmd = config.get_command_config("node").unwrap();
        assert_eq!(node_cmd.dev_bind, vec!["/dev/null", "/dev/random"]);
    }

    #[test]
    fn test_custom_template_names() {
        let yaml = r#"
templates:
  minimal:
    unshare:
      - network
  strict:
    unshare:
      - network
      - pid
      - ipc
    ro_bind:
      - /usr

commands:
  node:
    extends: minimal
    bind:
      - ~/.npm:~/.npm
  python:
    extends: strict
    bind:
      - ~/.local:~/.local
"#;
        let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();

        // Verify templates exist
        assert_eq!(config.templates.len(), 2);
        assert!(config.templates.contains_key("minimal"));
        assert!(config.templates.contains_key("strict"));

        // Test node with minimal template
        let node_cmd = config.get_command_config("node").unwrap();
        assert_eq!(node_cmd.extends, Some("minimal".to_string()));
        let merged_node = config.merge_with_template(node_cmd);
        assert_eq!(merged_node.unshare, vec!["network"]);
        assert_eq!(merged_node.bind, vec!["~/.npm:~/.npm"]);

        // Test python with strict template
        let python_cmd = config.get_command_config("python").unwrap();
        assert_eq!(python_cmd.extends, Some("strict".to_string()));
        let merged_python = config.merge_with_template(python_cmd);
        assert_eq!(merged_python.unshare, vec!["network", "pid", "ipc"]);
        assert_eq!(merged_python.ro_bind, vec!["/usr"]);
        assert_eq!(merged_python.bind, vec!["~/.local:~/.local"]);
    }

    #[test]
    fn test_nonexistent_template() {
        let yaml = r#"
templates:
  base:
    unshare:
      - network

commands:
  node:
    extends: nonexistent
    bind:
      - ~/.npm:~/.npm
"#;
        let config: BwrapConfig = serde_yaml::from_str(yaml).unwrap();
        let node_cmd = config.get_command_config("node").unwrap();
        let merged = config.merge_with_template(node_cmd.clone());

        // Should not merge anything, just return the original command config
        assert_eq!(merged.unshare, node_cmd.unshare);
        assert_eq!(merged.bind, node_cmd.bind);
    }
}
