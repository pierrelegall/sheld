use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub mod loader;

/// Custom deserializer for extends field that accepts both String and Vec<String>
fn deserialize_extends<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct ExtendsVisitor;

    impl<'de> Visitor<'de> for ExtendsVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or list of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value.to_string()])
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(value) = seq.next_element()? {
                vec.push(value);
            }
            Ok(vec)
        }
    }

    deserializer.deserialize_any(ExtendsVisitor)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
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
    #[serde(default = "default_override", rename = "override")]
    pub override_parent: bool,
    #[serde(default, deserialize_with = "deserialize_extends")]
    pub extends: Vec<String>,
    #[serde(default)]
    pub share: Vec<String>,
    #[serde(default)]
    pub bind: Vec<String>,
    #[serde(default)]
    pub ro_bind: Vec<String>,
    #[serde(default)]
    pub dev_bind: Vec<String>,
    #[serde(default)]
    pub bind_try: Vec<String>,
    #[serde(default)]
    pub ro_bind_try: Vec<String>,
    #[serde(default)]
    pub dev_bind_try: Vec<String>,
    #[serde(default)]
    pub tmpfs: Vec<String>,
    #[serde(default)]
    pub chdir: Option<String>,
    #[serde(default = "default_die_with_parent")]
    pub die_with_parent: bool,
    #[serde(default = "default_new_session")]
    pub new_session: bool,
    #[serde(default)]
    pub cap: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub unset_env: Vec<String>,
}

fn default_enabled() -> bool {
    true
}

fn default_override() -> bool {
    false
}

fn default_die_with_parent() -> bool {
    false
}

fn default_new_session() -> bool {
    false
}

/// Deduplicate a vector, preserving order (first occurrence kept)
fn deduplicate_vec(vec: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    vec.into_iter()
        .filter(|item| seen.insert(item.clone()))
        .collect()
}

impl Entry {
    /// Deep merge parent and child entries
    /// - Arrays: parent items first, then unique child items (deduplicated)
    /// - env HashMap: parent + child, child wins on conflicts
    /// - Scalar fields: child value wins
    /// - Empty child arrays preserve parent arrays
    pub fn deep_merge(parent: Entry, child: Entry) -> Entry {
        // Merge arrays: parent first, then unique child items
        let mut merged_share = parent.share.clone();
        merged_share.extend(child.share.clone());
        let merged_share = if child.share.is_empty() {
            parent.share
        } else {
            deduplicate_vec(merged_share)
        };

        let mut merged_bind = parent.bind.clone();
        merged_bind.extend(child.bind.clone());
        let merged_bind = if child.bind.is_empty() {
            parent.bind
        } else {
            deduplicate_vec(merged_bind)
        };

        let mut merged_ro_bind = parent.ro_bind.clone();
        merged_ro_bind.extend(child.ro_bind.clone());
        let merged_ro_bind = if child.ro_bind.is_empty() {
            parent.ro_bind
        } else {
            deduplicate_vec(merged_ro_bind)
        };

        let mut merged_dev_bind = parent.dev_bind.clone();
        merged_dev_bind.extend(child.dev_bind.clone());
        let merged_dev_bind = if child.dev_bind.is_empty() {
            parent.dev_bind
        } else {
            deduplicate_vec(merged_dev_bind)
        };

        let mut merged_tmpfs = parent.tmpfs.clone();
        merged_tmpfs.extend(child.tmpfs.clone());
        let merged_tmpfs = if child.tmpfs.is_empty() {
            parent.tmpfs
        } else {
            deduplicate_vec(merged_tmpfs)
        };

        let mut merged_unset_env = parent.unset_env.clone();
        merged_unset_env.extend(child.unset_env.clone());
        let merged_unset_env = if child.unset_env.is_empty() {
            parent.unset_env
        } else {
            deduplicate_vec(merged_unset_env)
        };

        // Merge env: parent + child, child wins on conflicts
        let mut merged_env = parent.env.clone();
        merged_env.extend(child.env);

        // Merge bind_try variants
        let mut merged_bind_try = parent.bind_try.clone();
        merged_bind_try.extend(child.bind_try.clone());
        let merged_bind_try = if child.bind_try.is_empty() {
            parent.bind_try
        } else {
            deduplicate_vec(merged_bind_try)
        };

        let mut merged_ro_bind_try = parent.ro_bind_try.clone();
        merged_ro_bind_try.extend(child.ro_bind_try.clone());
        let merged_ro_bind_try = if child.ro_bind_try.is_empty() {
            parent.ro_bind_try
        } else {
            deduplicate_vec(merged_ro_bind_try)
        };

        let mut merged_dev_bind_try = parent.dev_bind_try.clone();
        merged_dev_bind_try.extend(child.dev_bind_try.clone());
        let merged_dev_bind_try = if child.dev_bind_try.is_empty() {
            parent.dev_bind_try
        } else {
            deduplicate_vec(merged_dev_bind_try)
        };

        // Merge cap
        let mut merged_cap = parent.cap.clone();
        merged_cap.extend(child.cap.clone());
        let merged_cap = if child.cap.is_empty() {
            parent.cap
        } else {
            deduplicate_vec(merged_cap)
        };

        // Scalar fields: child wins (including chdir, die_with_parent, new_session)
        Entry {
            entry_type: child.entry_type,
            enabled: child.enabled,
            override_parent: child.override_parent,
            extends: child.extends,
            share: merged_share,
            bind: merged_bind,
            ro_bind: merged_ro_bind,
            dev_bind: merged_dev_bind,
            bind_try: merged_bind_try,
            ro_bind_try: merged_ro_bind_try,
            dev_bind_try: merged_dev_bind_try,
            tmpfs: merged_tmpfs,
            chdir: child.chdir.or(parent.chdir),
            die_with_parent: child.die_with_parent,
            new_session: child.new_session,
            cap: merged_cap,
            env: merged_env,
            unset_env: merged_unset_env,
        }
    }
}

impl Config {
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let config: Config = serde_yaml::from_str(yaml).context("Failed to parse YAML config")?;

        Ok(config)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let yaml = fs::read_to_string(path.as_ref())
            .context(format!("Failed to read config file: {:?}", path.as_ref()))?;

        let config: Config = serde_yaml::from_str(&yaml)
            .context(format!("Failed to parse YAML config {:?}", path.as_ref()))?;

        Ok(config)
    }

    /// Get all entries
    pub fn get_entries(&self) -> HashMap<String, Entry> {
        self.entries
            .iter()
            .map(|(name, entry)| (name.clone(), entry.clone().into()))
            .collect()
    }

    /// Get entries with constrains
    pub fn get_entries_with<F>(&self, predicate: F) -> HashMap<String, Entry>
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
    pub fn get_entry(&self, command: &str) -> Option<Entry> {
        self.entries.get(command).map(|entry| entry.clone().into())
    }

    /// Get an entry with constrains
    pub fn get_entry_with<F>(&self, name: &str, predicate: F) -> Option<Entry>
    where
        F: Fn(&Entry) -> bool,
    {
        self.entries
            .get(name)
            .filter(|entry| predicate(entry))
            .map(|entry| entry.clone().into())
    }

    /// Get all command entries (filtering by type: command)
    pub fn get_commands(&self) -> HashMap<String, Entry> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.entry_type == EntryType::Command)
            .map(|(name, entry)| (name.clone(), entry.clone().into()))
            .collect()
    }

    /// Get a specific command configuration
    pub fn get_command(&self, name: &str) -> Option<Entry> {
        self.entries
            .get(name)
            .filter(|entry| entry.entry_type == EntryType::Command)
            .map(|entry| entry.clone().into())
    }

    /// Get all model entries (filtering by type: command)
    pub fn get_models(&self) -> HashMap<String, Entry> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.entry_type == EntryType::Model)
            .map(|(name, entry)| (name.clone(), entry.clone().into()))
            .collect()
    }

    /// Get a model entry by name
    fn get_model(&self, name: &str) -> Option<Entry> {
        self.entries
            .get(name)
            .filter(|entry| entry.entry_type == EntryType::Model)
            .map(|entry| entry.clone().into())
    }

    /// Merge command config with its templates (if extends is set)
    /// Models are applied in order, with later models overriding earlier ones
    pub fn merge_with_template(&self, cmd_config: Entry) -> Entry {
        // Save the command's original values to apply at the end
        let cmd_share = cmd_config.share.clone();
        let cmd_bind = cmd_config.bind.clone();
        let cmd_ro_bind = cmd_config.ro_bind.clone();
        let cmd_dev_bind = cmd_config.dev_bind.clone();
        let cmd_bind_try = cmd_config.bind_try.clone();
        let cmd_ro_bind_try = cmd_config.ro_bind_try.clone();
        let cmd_dev_bind_try = cmd_config.dev_bind_try.clone();
        let cmd_tmpfs = cmd_config.tmpfs.clone();
        let cmd_unset_env = cmd_config.unset_env.clone();
        let cmd_env = cmd_config.env.clone();
        let cmd_cap = cmd_config.cap.clone();

        let mut result = Entry {
            entry_type: cmd_config.entry_type.clone(),
            enabled: cmd_config.enabled,
            override_parent: cmd_config.override_parent,
            extends: vec![], // Clear extends after processing
            share: vec![],
            bind: vec![],
            ro_bind: vec![],
            dev_bind: vec![],
            bind_try: vec![],
            ro_bind_try: vec![],
            dev_bind_try: vec![],
            tmpfs: vec![],
            chdir: cmd_config.chdir.clone(),
            die_with_parent: cmd_config.die_with_parent,
            new_session: cmd_config.new_session,
            cap: vec![],
            env: HashMap::new(),
            unset_env: vec![],
        };

        // Iterate over each model in the extends list
        for model_name in &cmd_config.extends {
            if let Some(template) = self.get_model(model_name) {
                // Extend arrays with template values
                result.share.extend(template.share.clone());
                result.bind.extend(template.bind.clone());
                result.ro_bind.extend(template.ro_bind.clone());
                result.dev_bind.extend(template.dev_bind.clone());
                result.bind_try.extend(template.bind_try.clone());
                result.ro_bind_try.extend(template.ro_bind_try.clone());
                result.dev_bind_try.extend(template.dev_bind_try.clone());
                result.tmpfs.extend(template.tmpfs.clone());
                result.unset_env.extend(template.unset_env.clone());
                result.cap.extend(template.cap.clone());

                // Merge env (later templates override earlier ones)
                result.env.extend(template.env.clone());
            }
            // If model doesn't exist, skip it (no error)
        }

        // Finally, apply command's own values (command values take precedence)
        result.share.extend(cmd_share);
        result.bind.extend(cmd_bind);
        result.ro_bind.extend(cmd_ro_bind);
        result.dev_bind.extend(cmd_dev_bind);
        result.bind_try.extend(cmd_bind_try);
        result.ro_bind_try.extend(cmd_ro_bind_try);
        result.dev_bind_try.extend(cmd_dev_bind_try);
        result.tmpfs.extend(cmd_tmpfs);
        result.unset_env.extend(cmd_unset_env);
        result.cap.extend(cmd_cap);
        result.env.extend(cmd_env);

        result
    }

    // Deprecated: use merge_with_template instead
    pub fn merge_with_base(&self, cmd_config: Entry) -> Entry {
        self.merge_with_template(cmd_config)
    }

    /// Merge another config into this one
    /// - Entries with the same name: depends on override field
    ///   - override: true -> child completely replaces parent
    ///   - override: false (default) -> deep merge parent and child
    /// - Special case: if child has enabled=false, skip merge and keep parent entry
    /// - Distinct entries: both are included
    pub fn merge(parent: Config, child: Config) -> Config {
        let mut merged_entries = parent.entries.clone();

        for (name, child_entry) in child.entries {
            // If child entry is disabled and parent has this entry, skip the child
            // (treat disabled in child as "use parent version instead")
            if !child_entry.enabled && merged_entries.contains_key(&name) {
                continue;
            }

            // Check if parent has an entry with the same name
            if let Some(parent_entry) = merged_entries.get(&name) {
                if child_entry.override_parent {
                    // override: true -> child completely replaces parent
                    merged_entries.insert(name, child_entry);
                } else {
                    // override: false (default) -> deep merge
                    let merged_entry = Entry::deep_merge(parent_entry.clone(), child_entry);
                    merged_entries.insert(name, merged_entry);
                }
            } else {
                // Parent doesn't have this entry, just add child entry
                merged_entries.insert(name, child_entry);
            }
        }

        Config {
            entries: merged_entries,
        }
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
        let config = Config::from_yaml(indoc! {"
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
        let config = Config::from_yaml(indoc! {"
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
        assert_eq!(node_cmd.extends, vec!["base"]);
        assert_eq!(node_cmd.bind, vec!["~/.npm:~/.npm"]);
    }

    #[test]
    fn test_get_command() {
        let config = Config::from_yaml(indoc! {"
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
        let config = Config::from_yaml(indoc! {"
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
        let config = Config::from_yaml(indoc! {"
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

        let config = Config::from_file(temp_file.path()).unwrap();
        let commands = config.get_commands();
        assert_eq!(commands.len(), 1);
        assert!(commands.contains_key("test"));
    }

    #[test]
    fn test_default_enabled() {
        let config = Config::from_yaml(indoc! {"
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
        let config = Config::from_yaml(indoc! {"
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
        let config = Config::from_yaml(indoc! {"
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
        let config = Config::from_yaml(indoc! {"
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
        let config = Config::from_yaml(indoc! {"
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
              bind:
                - ~/.local:~/.local
        "})
        .unwrap();

        // Verify we have 2 commands
        let commands = config.get_commands();
        assert_eq!(commands.len(), 2);

        // Test node with minimal template
        let node_cmd = config.get_command("node").unwrap();
        assert_eq!(node_cmd.extends, vec!["minimal"]);
        let merged_node = config.merge_with_template(node_cmd);
        assert_eq!(merged_node.share, vec!["user", "network"]);
        assert_eq!(merged_node.bind, vec!["~/.npm:~/.npm"]);

        // Test python with strict template
        let python_cmd = config.get_command("python").unwrap();
        assert_eq!(python_cmd.extends, vec!["strict"]);
        let merged_python = config.merge_with_template(python_cmd);
        assert_eq!(merged_python.share, vec!["user"]);
        assert_eq!(merged_python.ro_bind, vec!["/usr"]);
        assert_eq!(merged_python.bind, vec!["~/.local:~/.local"]);
    }

    #[test]
    fn test_nonexistent_template() {
        let config = Config::from_yaml(indoc! {"
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
        let config = Config::from_yaml(indoc! {"
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
        let extends_base = config.get_entries_with(|e| e.extends.contains(&"base".to_string()));
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
        let config = Config::from_yaml(indoc! {"
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
        let config = Config::from_yaml(indoc! {"
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
        let config = Config::from_yaml(indoc! {"
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

    #[test]
    fn test_merge_both_configs_with_distinct_entries() {
        let user_config = Config::from_yaml(indoc! {"
            python:
              enabled: true
              share:
                - user
        "})
        .unwrap();

        let local_config = Config::from_yaml(indoc! {"
            node:
              enabled: true
              share:
                - network
        "})
        .unwrap();

        let merged = Config::merge(user_config, local_config);
        let commands = merged.get_commands();

        assert_eq!(commands.len(), 2);
        assert!(commands.contains_key("python"));
        assert!(commands.contains_key("node"));
    }

    #[test]
    fn test_merge_local_command_overrides_user_command() {
        let user_config = Config::from_yaml(indoc! {"
            node:
              enabled: true
              share:
                - user
        "})
        .unwrap();

        let local_config = Config::from_yaml(indoc! {"
            node:
              enabled: true
              override: true
              share:
                - network
        "})
        .unwrap();

        let merged = Config::merge(user_config, local_config);
        let node_cmd = merged.get_command("node").unwrap();

        // Local config should win (due to override: true)
        assert_eq!(node_cmd.share, vec!["network"]);
    }

    #[test]
    fn test_merge_local_command_extends_user_model() {
        let user_config = Config::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user
              ro_bind:
                - /usr
        "})
        .unwrap();

        let local_config = Config::from_yaml(indoc! {"
            node:
              extends: base
              bind:
                - ~/.npm:~/.npm
        "})
        .unwrap();

        let merged = Config::merge(user_config, local_config);
        let node_cmd = merged.get_command("node").unwrap();
        let with_template = merged.merge_with_template(node_cmd);

        // Should inherit from user's base model
        assert_eq!(with_template.share, vec!["user"]);
        assert_eq!(with_template.ro_bind, vec!["/usr"]);
        assert_eq!(with_template.bind, vec!["~/.npm:~/.npm"]);
    }

    #[test]
    fn test_merge_local_model_shadows_user_model() {
        let user_config = Config::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user
        "})
        .unwrap();

        let local_config = Config::from_yaml(indoc! {"
            base:
              type: model
              override: true
              share:
                - network
        "})
        .unwrap();

        let merged = Config::merge(user_config, local_config);
        let base_model = merged.get_model("base").unwrap();

        // Local model should completely replace user model (due to override: true)
        assert_eq!(base_model.share, vec!["network"]);
    }

    #[test]
    fn test_merge_local_disabled_uses_user_version() {
        let user_config = Config::from_yaml(indoc! {"
            node:
              enabled: true
              share:
                - user
        "})
        .unwrap();

        let local_config = Config::from_yaml(indoc! {"
            node:
              enabled: false
              share:
                - network
        "})
        .unwrap();

        let merged = Config::merge(user_config, local_config);
        let node_cmd = merged.get_command("node").unwrap();

        // User version should be kept when local has enabled:false
        assert!(node_cmd.enabled);
        assert_eq!(node_cmd.share, vec!["user"]);
    }

    #[test]
    fn test_merge_only_user_config() {
        let user_config = Config::from_yaml(indoc! {"
            node:
              enabled: true
              share:
                - user
        "})
        .unwrap();

        let empty_config = Config::from_yaml("").unwrap();

        let merged = Config::merge(user_config, empty_config);
        let commands = merged.get_commands();

        assert_eq!(commands.len(), 1);
        assert!(commands.contains_key("node"));
    }

    #[test]
    fn test_merge_only_local_config() {
        let empty_config = Config::from_yaml("").unwrap();

        let local_config = Config::from_yaml(indoc! {"
            node:
              enabled: true
              share:
                - network
        "})
        .unwrap();

        let merged = Config::merge(empty_config, local_config);
        let commands = merged.get_commands();

        assert_eq!(commands.len(), 1);
        assert!(commands.contains_key("node"));
    }

    #[test]
    fn test_override_defaults_to_false() {
        let config = Config::from_yaml(indoc! {"
            node:
              enabled: true
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        assert_eq!(node_cmd.override_parent, false);
    }

    #[test]
    fn test_override_true_replaces_parent() {
        let parent_config = Config::from_yaml(indoc! {"
            node:
              share:
                - user
                - pid
              bind:
                - /usr:/usr
        "})
        .unwrap();

        let child_config = Config::from_yaml(indoc! {"
            node:
              override: true
              share:
                - network
        "})
        .unwrap();

        let merged = Config::merge(parent_config, child_config);
        let node_cmd = merged.get_command("node").unwrap();

        // Child completely replaces parent
        assert_eq!(node_cmd.share, vec!["network"]);
        assert!(node_cmd.bind.is_empty());
    }

    #[test]
    fn test_override_false_deep_merges_arrays() {
        let parent_config = Config::from_yaml(indoc! {"
            node:
              share:
                - user
                - pid
        "})
        .unwrap();

        let child_config = Config::from_yaml(indoc! {"
            node:
              override: false
              share:
                - network
                - pid
        "})
        .unwrap();

        let merged = Config::merge(parent_config, child_config);
        let node_cmd = merged.get_command("node").unwrap();

        // Arrays merged and deduplicated (parent first, then unique child items)
        assert_eq!(node_cmd.share, vec!["user", "pid", "network"]);
    }

    #[test]
    fn test_override_false_merges_env() {
        let parent_config = Config::from_yaml(indoc! {"
            node:
              env:
                FROM: parent
                KEEP: this
        "})
        .unwrap();

        let child_config = Config::from_yaml(indoc! {"
            node:
              env:
                FROM: child
                NEW: value
        "})
        .unwrap();

        let merged = Config::merge(parent_config, child_config);
        let node_cmd = merged.get_command("node").unwrap();

        // Env merged, child wins on conflicts
        assert_eq!(node_cmd.env.get("FROM"), Some(&"child".to_string()));
        assert_eq!(node_cmd.env.get("KEEP"), Some(&"this".to_string()));
        assert_eq!(node_cmd.env.get("NEW"), Some(&"value".to_string()));
    }

    #[test]
    fn test_empty_child_arrays_preserve_parent() {
        let parent_config = Config::from_yaml(indoc! {"
            node:
              share:
                - user
                - network
        "})
        .unwrap();

        let child_config = Config::from_yaml(indoc! {"
            node:
              share: []
        "})
        .unwrap();

        let merged = Config::merge(parent_config, child_config);
        let node_cmd = merged.get_command("node").unwrap();

        // Empty child array preserves parent array
        assert_eq!(node_cmd.share, vec!["user", "network"]);
    }

    #[test]
    fn test_enabled_false_with_override_false() {
        let parent_config = Config::from_yaml(indoc! {"
            node:
              share:
                - user
        "})
        .unwrap();

        let child_config = Config::from_yaml(indoc! {"
            node:
              enabled: false
              override: false
              share:
                - network
        "})
        .unwrap();

        let merged = Config::merge(parent_config, child_config);
        let node_cmd = merged.get_command("node").unwrap();

        // enabled: false takes precedence, parent entry is used
        assert!(node_cmd.enabled);
        assert_eq!(node_cmd.share, vec!["user"]);
    }

    #[test]
    fn test_enabled_false_with_override_true() {
        let parent_config = Config::from_yaml(indoc! {"
            node:
              share:
                - user
        "})
        .unwrap();

        let child_config = Config::from_yaml(indoc! {"
            node:
              enabled: false
              override: true
              share:
                - network
        "})
        .unwrap();

        let merged = Config::merge(parent_config, child_config);
        let node_cmd = merged.get_command("node").unwrap();

        // enabled: false takes precedence regardless of override value
        assert!(node_cmd.enabled);
        assert_eq!(node_cmd.share, vec!["user"]);
    }

    #[test]
    fn test_extends_single_string_syntax() {
        let config = Config::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user

            node:
              extends: base
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        assert_eq!(node_cmd.extends, vec!["base"]);
    }

    #[test]
    fn test_extends_list_syntax_multiple_models() {
        let config = Config::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user

            network:
              type: model
              share:
                - network

            node:
              extends: [base, network]
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        assert_eq!(node_cmd.extends, vec!["base", "network"]);
    }

    #[test]
    fn test_extends_models_applied_in_order() {
        let config = Config::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user
              ro_bind:
                - /usr

            network:
              type: model
              share:
                - network
              ro_bind:
                - /etc/resolv.conf

            node:
              extends: [base, network]
              bind:
                - ~/.npm:~/.npm
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_template(node_cmd);

        // Should have shares from both models
        assert!(merged.share.contains(&"user".to_string()));
        assert!(merged.share.contains(&"network".to_string()));

        // Should have ro_bind from both models (base first, then network)
        assert!(merged.ro_bind.contains(&"/usr".to_string()));
        assert!(merged.ro_bind.contains(&"/etc/resolv.conf".to_string()));

        // Should have bind from command itself
        assert!(merged.bind.contains(&"~/.npm:~/.npm".to_string()));
    }

    #[test]
    fn test_extends_later_model_overrides_earlier_env() {
        let config = Config::from_yaml(indoc! {"
            base:
              type: model
              env:
                KEY: base_value
                OTHER: keep_this

            override:
              type: model
              env:
                KEY: override_value

            node:
              extends: [base, override]
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_template(node_cmd);

        // Later model's env should override earlier model's env
        assert_eq!(merged.env.get("KEY"), Some(&"override_value".to_string()));
        // Env from first model that wasn't overridden should remain
        assert_eq!(merged.env.get("OTHER"), Some(&"keep_this".to_string()));
    }

    #[test]
    fn test_extends_entry_settings_override_all_models() {
        let config = Config::from_yaml(indoc! {"
            base:
              type: model
              env:
                KEY: base_value

            network:
              type: model
              env:
                KEY: network_value

            node:
              extends: [base, network]
              env:
                KEY: command_value
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_template(node_cmd);

        // Command's own env should override all models
        assert_eq!(merged.env.get("KEY"), Some(&"command_value".to_string()));
    }

    #[test]
    fn test_extends_skip_nonexistent_model() {
        let config = Config::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user

            network:
              type: model
              share:
                - network

            node:
              extends: [base, nonexistent, network]
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_template(node_cmd);

        // Should apply base and network, skip nonexistent
        assert!(merged.share.contains(&"user".to_string()));
        assert!(merged.share.contains(&"network".to_string()));
    }

    #[test]
    fn test_extends_all_models_nonexistent() {
        let config = Config::from_yaml(indoc! {"
            node:
              extends: [foo, bar]
              share:
                - user
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_template(node_cmd);

        // Should just have command's own settings
        assert_eq!(merged.share, vec!["user"]);
    }

    #[test]
    fn test_extends_empty_list() {
        let config = Config::from_yaml(indoc! {"
            node:
              extends: []
              share:
                - user
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        assert_eq!(node_cmd.extends, Vec::<String>::new());

        let merged = config.merge_with_template(node_cmd);
        assert_eq!(merged.share, vec!["user"]);
    }
}
