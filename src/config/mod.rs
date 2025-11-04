use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub mod loader;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BwrapConfig {
    #[serde(flatten)]
    pub entries: HashMap<String, Entry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    Command,
    Model,
}

impl Default for EntryType {
    fn default() -> Self {
        EntryType::Command
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    #[serde(default, rename = "type")]
    pub entry_type: EntryType,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub extends: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub extends: Option<String>,
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

impl From<Entry> for CommandConfig {
    fn from(entry: Entry) -> Self {
        CommandConfig {
            enabled: entry.enabled,
            extends: entry.extends,
            share: entry.share,
            bind: entry.bind,
            ro_bind: entry.ro_bind,
            dev_bind: entry.dev_bind,
            tmpfs: entry.tmpfs,
            env: entry.env,
            unset_env: entry.unset_env,
        }
    }
}

impl BwrapConfig {
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let config: BwrapConfig =
            serde_yaml::from_str(yaml).context("Failed to parse YAML config")?;

        Ok(config)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let yaml = fs::read_to_string(path.as_ref())
            .context(format!("Failed to read config file: {:?}", path.as_ref()))?;

        let config: BwrapConfig = serde_yaml::from_str(&yaml)
            .context(format!("Failed to parse YAML config {:?}", path.as_ref()))?;

        Ok(config)
    }

    /// Get all entries
    pub fn get_entries(&self) -> HashMap<String, CommandConfig> {
        self.entries
            .iter()
            .map(|(name, entry)| (name.clone(), entry.clone().into()))
            .collect()
    }

    /// Get entries with constrains
    pub fn get_entries_with<F>(&self, predicate: F) -> HashMap<String, CommandConfig>
    where
        F: Fn(&Entry) -> bool,
    {
        self.entries
            .iter()
            .filter(|(_, entry)| predicate(entry))
            .map(|(name, entry)| (name.clone(), entry.clone().into()))
            .collect()
    }

    /// Get a specific command configuration
    pub fn get_entry(&self, command: &str) -> Option<CommandConfig> {
        self.entries.get(command).map(|entry| entry.clone().into())
    }

    /// Get an entry with constrains
    pub fn get_entry_with<F>(&self, name: &str, predicate: F) -> Option<CommandConfig>
    where
        F: Fn(&Entry) -> bool,
    {
        self.entries
            .get(name)
            .filter(|entry| predicate(entry))
            .map(|entry| entry.clone().into())
    }

    /// Get all command entries (filtering by type: command)
    pub fn get_commands(&self) -> HashMap<String, CommandConfig> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.entry_type == EntryType::Command)
            .map(|(name, entry)| (name.clone(), entry.clone().into()))
            .collect()
    }

    /// Get a specific command configuration
    pub fn get_command(&self, name: &str) -> Option<CommandConfig> {
        self.entries
            .get(name)
            .filter(|entry| entry.entry_type == EntryType::Command)
            .map(|entry| entry.clone().into())
    }

    /// Get all model entries (filtering by type: command)
    pub fn get_models(&self) -> HashMap<String, CommandConfig> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.entry_type == EntryType::Model)
            .map(|(name, entry)| (name.clone(), entry.clone().into()))
            .collect()
    }

    /// Get a model entry by name
    fn get_model(&self, name: &str) -> Option<CommandConfig> {
        self.entries
            .get(name)
            .filter(|entry| entry.entry_type == EntryType::Model)
            .map(|entry| entry.clone().into())
    }

    /// Merge command config with its template (if extends is set)
    pub fn merge_with_template(&self, mut cmd_config: CommandConfig) -> CommandConfig {
        if let Some(extends) = &cmd_config.extends {
            if let Some(template) = self.get_model(extends) {
                // Merge template config into command config
                cmd_config.share.extend(template.share.clone());
                cmd_config.bind.extend(template.bind.clone());
                cmd_config.ro_bind.extend(template.ro_bind.clone());
                cmd_config.dev_bind.extend(template.dev_bind.clone());
                cmd_config.tmpfs.extend(template.tmpfs.clone());
                // Merge env vars (command-specific takes precedence)
                for (key, value) in template.env.iter() {
                    cmd_config.env.entry(key.clone()).or_insert(value.clone());
                }
                cmd_config.unset_env.extend(template.unset_env.clone());
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
    use indoc::indoc;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_basic_config() {
        let config = BwrapConfig::from_yaml(indoc! {"
            node:
              enabled: true
              share:
                - user
                - network
              bind:
                - ~/.npm:~/.npm
        "})
        .unwrap();
        let commands = config.get_commands();
        assert_eq!(commands.len(), 1);
        assert!(commands.contains_key("node"));

        let node_cmd = commands.get("node").unwrap();
        assert!(node_cmd.enabled);
        assert_eq!(node_cmd.share, vec!["user", "network"]);
        assert_eq!(node_cmd.bind, vec!["~/.npm:~/.npm"]);
    }

    #[test]
    fn test_parse_config_with_base() {
        let config = BwrapConfig::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user
              ro_bind:
                - /usr
                - /lib

            node:
              extends: base
              bind:
                - ~/.npm:~/.npm
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        assert_eq!(node_cmd.extends, Some("base".to_string()));
        assert_eq!(node_cmd.bind, vec!["~/.npm:~/.npm"]);
    }

    #[test]
    fn test_get_command() {
        let config = BwrapConfig::from_yaml(indoc! {"
            node:
              enabled: true
            python:
              enabled: false
        "})
        .unwrap();

        assert!(config.get_command("node").is_some());
        assert!(config.get_command("python").is_some());
        assert!(config.get_command("ruby").is_none());
    }

    #[test]
    fn test_merge_with_base() {
        let config = BwrapConfig::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user
              ro_bind:
                - /usr

            node:
              extends: base
              bind:
                - ~/.npm:~/.npm
        "})
        .unwrap();
        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_base(node_cmd);

        // Should have both base and command-specific settings
        assert_eq!(merged.share, vec!["user"]);
        assert_eq!(merged.ro_bind, vec!["/usr"]);
        assert_eq!(merged.bind, vec!["~/.npm:~/.npm"]);
    }

    #[test]
    fn test_merge_without_extends() {
        let config = BwrapConfig::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user

            node:
              bind:
                - ~/.npm:~/.npm
        "})
        .unwrap();
        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_base(node_cmd.clone());

        // Should not merge base since extends is not set
        assert_eq!(merged.share, node_cmd.share);
        assert_eq!(merged.bind, node_cmd.bind);
    }

    #[test]
    fn test_from_file() {
        let yaml = indoc! {"
            test:
              enabled: true
        "};
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();

        let config = BwrapConfig::from_file(temp_file.path()).unwrap();
        let commands = config.get_commands();
        assert_eq!(commands.len(), 1);
        assert!(commands.contains_key("test"));
    }

    #[test]
    fn test_default_enabled() {
        let config = BwrapConfig::from_yaml(indoc! {"
            node:
              share:
                - user
        "})
        .unwrap();
        let node_cmd = config.get_command("node").unwrap();
        // enabled should default to true
        assert!(node_cmd.enabled);
    }

    #[test]
    fn test_disabled_command() {
        let config = BwrapConfig::from_yaml(indoc! {"
            node:
              enabled: false
              share:
                - user
        "})
        .unwrap();
        let node_cmd = config.get_command("node").unwrap();
        assert!(!node_cmd.enabled);
    }

    #[test]
    fn test_env_variables() {
        let config = BwrapConfig::from_yaml(indoc! {"
            node:
              env:
                NODE_ENV: production
                PATH: /custom/path
              unset_env:
                - DEBUG
        "})
        .unwrap();
        let node_cmd = config.get_command("node").unwrap();

        assert_eq!(node_cmd.env.len(), 2);
        assert_eq!(
            node_cmd.env.get("NODE_ENV"),
            Some(&"production".to_string())
        );
        assert_eq!(node_cmd.unset_env, vec!["DEBUG"]);
    }

    #[test]
    fn test_tmpfs() {
        let config = BwrapConfig::from_yaml(indoc! {"
            node:
              tmpfs:
                - /tmp
                - /var/tmp
        "})
        .unwrap();
        let node_cmd = config.get_command("node").unwrap();
        assert_eq!(node_cmd.tmpfs, vec!["/tmp", "/var/tmp"]);
    }

    #[test]
    fn test_dev_bind() {
        let config = BwrapConfig::from_yaml(indoc! {"
            node:
              dev_bind:
                - /dev/null
                - /dev/random
        "})
        .unwrap();
        let node_cmd = config.get_command("node").unwrap();
        assert_eq!(node_cmd.dev_bind, vec!["/dev/null", "/dev/random"]);
    }

    #[test]
    fn test_custom_template_names() {
        let config = BwrapConfig::from_yaml(indoc! {"
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
              bind:
                - ~/.local:~/.local
        "})
        .unwrap();

        // Verify we have 2 commands
        let commands = config.get_commands();
        assert_eq!(commands.len(), 2);

        // Test node with minimal template
        let node_cmd = config.get_command("node").unwrap();
        assert_eq!(node_cmd.extends, Some("minimal".to_string()));
        let merged_node = config.merge_with_template(node_cmd);
        assert_eq!(merged_node.share, vec!["user", "network"]);
        assert_eq!(merged_node.bind, vec!["~/.npm:~/.npm"]);

        // Test python with strict template
        let python_cmd = config.get_command("python").unwrap();
        assert_eq!(python_cmd.extends, Some("strict".to_string()));
        let merged_python = config.merge_with_template(python_cmd);
        assert_eq!(merged_python.share, vec!["user"]);
        assert_eq!(merged_python.ro_bind, vec!["/usr"]);
        assert_eq!(merged_python.bind, vec!["~/.local:~/.local"]);
    }

    #[test]
    fn test_nonexistent_template() {
        let config = BwrapConfig::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user

            node:
              extends: nonexistent
              bind:
                - ~/.npm:~/.npm
        "})
        .unwrap();
        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_template(node_cmd.clone());

        // Should not merge anything, just return the original command config
        assert_eq!(merged.share, node_cmd.share);
        assert_eq!(merged.bind, node_cmd.bind);
    }

    #[test]
    fn test_get_entries_with() {
        let config = BwrapConfig::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user

            node:
              enabled: true
              extends: base
              bind:
                - ~/.npm:~/.npm

            python:
              enabled: false
              extends: base
              bind:
                - ~/.local:~/.local

            rust:
              enabled: true
              extends: base
              share:
                - network
        "})
        .unwrap();

        // Filter enabled commands only
        let enabled = config.get_entries_with(|e| e.enabled && e.entry_type == EntryType::Command);
        assert_eq!(enabled.len(), 2);
        assert!(enabled.contains_key("node"));
        assert!(enabled.contains_key("rust"));
        assert!(!enabled.contains_key("python"));
        assert!(!enabled.contains_key("base"));

        // Filter disabled commands
        let disabled =
            config.get_entries_with(|e| !e.enabled && e.entry_type == EntryType::Command);
        assert_eq!(disabled.len(), 1);
        assert!(disabled.contains_key("python"));

        // Filter models
        let models = config.get_entries_with(|e| e.entry_type == EntryType::Model);
        assert_eq!(models.len(), 1);
        assert!(models.contains_key("base"));

        // Filter entries with network share
        let with_network = config.get_entries_with(|e| e.share.contains(&"network".to_string()));
        assert_eq!(with_network.len(), 1);
        assert!(with_network.contains_key("rust"));

        // Filter entries that extend base
        let extends_base = config.get_entries_with(|e| e.extends == Some("base".to_string()));
        assert_eq!(extends_base.len(), 3);

        // Complex filter: enabled commands with bind
        let enabled_with_bind = config.get_entries_with(|e| {
            e.enabled && e.entry_type == EntryType::Command && !e.bind.is_empty()
        });
        assert_eq!(enabled_with_bind.len(), 1);
        assert!(enabled_with_bind.contains_key("node"));
        assert!(!enabled_with_bind.contains_key("rust")); // rust has no bind
    }

    #[test]
    fn test_get_entry_with() {
        let config = BwrapConfig::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user

            node:
              enabled: true
              extends: base
              share:
                - network
              bind:
                - ~/.npm:~/.npm

            python:
              enabled: false
              extends: base
        "})
        .unwrap();

        // Get entry only if enabled
        let node_enabled = config.get_entry_with("node", |e| e.enabled);
        assert!(node_enabled.is_some());
        assert!(node_enabled.unwrap().enabled);

        let python_enabled = config.get_entry_with("python", |e| e.enabled);
        assert!(python_enabled.is_none());

        // Get entry only if it's a command
        let node_cmd = config.get_entry_with("node", |e| e.entry_type == EntryType::Command);
        assert!(node_cmd.is_some());

        let base_cmd = config.get_entry_with("base", |e| e.entry_type == EntryType::Command);
        assert!(base_cmd.is_none());

        // Get entry only if it's a model
        let base_model = config.get_entry_with("base", |e| e.entry_type == EntryType::Model);
        assert!(base_model.is_some());

        // Get entry with network share
        let node_network =
            config.get_entry_with("node", |e| e.share.contains(&"network".to_string()));
        assert!(node_network.is_some());

        let python_network =
            config.get_entry_with("python", |e| e.share.contains(&"network".to_string()));
        assert!(python_network.is_none());

        // Complex filter: enabled command with bind
        let node_complex = config.get_entry_with("node", |e| {
            e.enabled && e.entry_type == EntryType::Command && !e.bind.is_empty()
        });
        assert!(node_complex.is_some());

        let python_complex = config.get_entry_with("python", |e| {
            e.enabled && e.entry_type == EntryType::Command && !e.bind.is_empty()
        });
        assert!(python_complex.is_none());

        // Non-existent entry
        let nonexistent = config.get_entry_with("nonexistent", |_| true);
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_get_entries_with_empty_results() {
        let config = BwrapConfig::from_yaml(indoc! {"
            node:
              enabled: true
        "})
        .unwrap();

        // Filter that matches nothing
        let no_models = config.get_entries_with(|e| e.entry_type == EntryType::Model);
        assert_eq!(no_models.len(), 0);

        let no_network = config.get_entries_with(|e| e.share.contains(&"network".to_string()));
        assert_eq!(no_network.len(), 0);
    }

    #[test]
    fn test_get_entries_with_all_match() {
        let config = BwrapConfig::from_yaml(indoc! {"
            node:
              enabled: true
            python:
              enabled: true
            rust:
              enabled: true
        "})
        .unwrap();

        // Filter that matches everything
        let all = config.get_entries_with(|_| true);
        assert_eq!(all.len(), 3);

        let all_enabled = config.get_entries_with(|e| e.enabled);
        assert_eq!(all_enabled.len(), 3);
    }
}
