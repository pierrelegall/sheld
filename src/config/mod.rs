use anyhow::{Context, Result, bail};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub mod loader;

/// Custom deserializer for includes field that accepts both String and Vec<String>
fn deserialize_includes<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct IncludesVisitor;

    impl<'de> Visitor<'de> for IncludesVisitor {
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

    deserializer.deserialize_any(IncludesVisitor)
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawMount {
    Path(String),
    Pair((String, String)),
}

fn serialize_flexible_bind<S>(
    binds: &HashMap<String, String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use serde::ser::SerializeSeq;

    let mut sequence = serializer.serialize_seq(Some(binds.len()))?;
    for (dst, src) in binds {
        if src == dst {
            sequence.serialize_element(src)?;
        } else {
            sequence.serialize_element(&(src, dst))?;
        }
    }
    sequence.end()
}

#[derive(Debug, Clone, Serialize)]
pub struct Config {
    #[serde(flatten)]
    pub entries: HashMap<String, Entry>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    #[default]
    Command,
    Model,
}

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    #[serde(default, rename = "type")]
    pub entry_type: EntryType,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_override", rename = "override")]
    pub override_parent: bool,
    #[serde(default, deserialize_with = "deserialize_includes")]
    pub includes: Vec<String>,
    #[serde(default)]
    pub share: HashSet<String>,
    #[serde(default, serialize_with = "serialize_flexible_bind")]
    pub bind: HashMap<String, String>,
    #[serde(default, serialize_with = "serialize_flexible_bind")]
    pub ro_bind: HashMap<String, String>,
    #[serde(default, serialize_with = "serialize_flexible_bind")]
    pub dev_bind: HashMap<String, String>,
    #[serde(default, serialize_with = "serialize_flexible_bind")]
    pub bind_try: HashMap<String, String>,
    #[serde(default, serialize_with = "serialize_flexible_bind")]
    pub ro_bind_try: HashMap<String, String>,
    #[serde(default, serialize_with = "serialize_flexible_bind")]
    pub dev_bind_try: HashMap<String, String>,
    #[serde(default)]
    pub tmpfs: HashSet<String>,
    #[serde(default)]
    pub chdir: Option<String>,
    #[serde(default = "default_die_with_parent")]
    pub die_with_parent: bool,
    #[serde(default = "default_new_session")]
    pub new_session: bool,
    #[serde(default)]
    pub cap: HashSet<String>,
    #[serde(default)]
    pub setenv_if_unset: HashMap<String, String>,
    #[serde(default)]
    pub setenv: HashMap<String, String>,
    #[serde(default)]
    pub unsetenv: HashSet<String>,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(flatten)]
    entries: HashMap<String, RawEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    #[serde(default, rename = "type")]
    entry_type: EntryType,
    #[serde(default = "default_enabled")]
    enabled: bool,
    #[serde(default = "default_override", rename = "override")]
    override_parent: bool,
    #[serde(default, deserialize_with = "deserialize_includes")]
    includes: Vec<String>,
    #[serde(default)]
    share: HashSet<String>,
    #[serde(default)]
    bind: Vec<RawMount>,
    #[serde(default)]
    ro_bind: Vec<RawMount>,
    #[serde(default)]
    dev_bind: Vec<RawMount>,
    #[serde(default)]
    bind_try: Vec<RawMount>,
    #[serde(default)]
    ro_bind_try: Vec<RawMount>,
    #[serde(default)]
    dev_bind_try: Vec<RawMount>,
    #[serde(default)]
    tmpfs: HashSet<String>,
    #[serde(default)]
    chdir: Option<String>,
    #[serde(default = "default_die_with_parent")]
    die_with_parent: bool,
    #[serde(default = "default_new_session")]
    new_session: bool,
    #[serde(default)]
    cap: HashSet<String>,
    #[serde(default)]
    setenv_if_unset: HashMap<String, String>,
    #[serde(default)]
    setenv: HashMap<String, String>,
    #[serde(default)]
    unsetenv: HashSet<String>,
    #[serde(default)]
    alias: Option<String>,
    #[serde(default)]
    args: Vec<String>,
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(component) => normalized.push(component),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn resolve_path(raw: &str, base_dir: &Path, entry: &str, field: &str) -> Result<String> {
    let expanded = shellexpand::full(raw)
        .with_context(|| format!("Failed to expand {field} path {raw:?} in entry {entry:?}"))?;
    let path = Path::new(expanded.as_ref());
    let absolute = if path.is_relative() {
        base_dir.join(path)
    } else {
        path.to_path_buf()
    };

    let normalized = normalize_path(&absolute);
    if !normalized.is_absolute() {
        bail!("Resolved {field} path {raw:?} in entry {entry:?} is not absolute");
    }
    normalized.into_os_string().into_string().map_err(|path| {
        anyhow::anyhow!(
            "Resolved {field} path {:?} in entry {entry:?} is not valid UTF-8",
            path
        )
    })
}

fn resolve_mounts(
    mounts: Vec<RawMount>,
    base_dir: &Path,
    entry: &str,
    field: &str,
) -> Result<HashMap<String, String>> {
    let mut resolved = HashMap::new();
    for mount in mounts {
        let (src, dst) = match mount {
            RawMount::Path(path) => (path.clone(), path),
            RawMount::Pair((src, dst)) => (src, dst),
        };
        let dst = resolve_path(&dst, base_dir, entry, field)?;
        let src = resolve_path(&src, base_dir, entry, field)?;
        resolved.insert(dst, src);
    }
    Ok(resolved)
}

impl RawEntry {
    fn resolve(self, base_dir: &Path, entry: &str) -> Result<Entry> {
        Ok(Entry {
            entry_type: self.entry_type,
            enabled: self.enabled,
            override_parent: self.override_parent,
            includes: self.includes,
            share: self.share,
            bind: resolve_mounts(self.bind, base_dir, entry, "bind")?,
            ro_bind: resolve_mounts(self.ro_bind, base_dir, entry, "ro_bind")?,
            dev_bind: resolve_mounts(self.dev_bind, base_dir, entry, "dev_bind")?,
            bind_try: resolve_mounts(self.bind_try, base_dir, entry, "bind_try")?,
            ro_bind_try: resolve_mounts(self.ro_bind_try, base_dir, entry, "ro_bind_try")?,
            dev_bind_try: resolve_mounts(self.dev_bind_try, base_dir, entry, "dev_bind_try")?,
            tmpfs: self
                .tmpfs
                .into_iter()
                .map(|path| resolve_path(&path, base_dir, entry, "tmpfs"))
                .collect::<Result<_>>()?,
            chdir: self
                .chdir
                .map(|path| resolve_path(&path, base_dir, entry, "chdir"))
                .transpose()?,
            die_with_parent: self.die_with_parent,
            new_session: self.new_session,
            cap: self.cap,
            setenv_if_unset: self.setenv_if_unset,
            setenv: self.setenv,
            unsetenv: self.unsetenv,
            alias: self.alias,
            args: self.args,
        })
    }
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

impl Entry {
    /// Deep merge parent and child entries
    /// - Sets: parent and child items are combined
    /// - HashMaps: parent + child, child wins on conflicts
    /// - Scalar fields: child value wins
    /// - Empty child arrays preserve parent arrays
    pub fn deep_merge(parent: Entry, child: Entry) -> Entry {
        // Merge sets and maps with child values winning on conflicts.
        let mut merged_share = parent.share.clone();
        merged_share.extend(child.share);

        let mut merged_bind = parent.bind.clone();
        merged_bind.extend(child.bind);

        let mut merged_ro_bind = parent.ro_bind.clone();
        merged_ro_bind.extend(child.ro_bind);

        let mut merged_dev_bind = parent.dev_bind.clone();
        merged_dev_bind.extend(child.dev_bind);

        let mut merged_tmpfs = parent.tmpfs.clone();
        merged_tmpfs.extend(child.tmpfs);

        let mut merged_unsetenv = parent.unsetenv.clone();
        merged_unsetenv.extend(child.unsetenv);

        let mut merged_setenv_if_unset = parent.setenv_if_unset.clone();
        merged_setenv_if_unset.extend(child.setenv_if_unset);

        let mut merged_setenv = parent.setenv.clone();
        merged_setenv.extend(child.setenv);

        // Merge bind_try variants.
        let mut merged_bind_try = parent.bind_try.clone();
        merged_bind_try.extend(child.bind_try);

        let mut merged_ro_bind_try = parent.ro_bind_try.clone();
        merged_ro_bind_try.extend(child.ro_bind_try);

        let mut merged_dev_bind_try = parent.dev_bind_try.clone();
        merged_dev_bind_try.extend(child.dev_bind_try);

        // Merge cap.
        let mut merged_cap = parent.cap.clone();
        merged_cap.extend(child.cap);

        // Scalar fields: child wins (including chdir, die_with_parent, new_session)
        // `alias` is not merged
        Entry {
            entry_type: child.entry_type,
            enabled: child.enabled,
            override_parent: child.override_parent,
            includes: child.includes,
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
            setenv_if_unset: merged_setenv_if_unset,
            setenv: merged_setenv,
            unsetenv: merged_unsetenv,
            alias: child.alias,
            args: child.args,
        }
    }
}

impl Config {
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let current_dir = std::env::current_dir().context("Failed to get current directory")?;
        Self::from_yaml_with_base_dir(yaml, &current_dir)
    }

    fn from_yaml_with_base_dir(yaml: &str, base_dir: &Path) -> Result<Self> {
        let raw: RawConfig = serde_yaml::from_str(yaml).context("Failed to parse YAML config")?;
        let entries = raw
            .entries
            .into_iter()
            .map(|(name, entry)| entry.resolve(base_dir, &name).map(|entry| (name, entry)))
            .collect::<Result<_>>()?;

        Ok(Self { entries })
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .context("Failed to get current directory")?
                .join(path)
        };
        let absolute_path = normalize_path(&absolute_path);
        let yaml = fs::read_to_string(&absolute_path)
            .with_context(|| format!("Failed to read config file: {absolute_path:?}"))?;

        let base_dir = absolute_path
            .parent()
            .context("Configuration path has no parent directory")?;
        Self::from_yaml_with_base_dir(&yaml, base_dir)
            .with_context(|| format!("Failed to parse YAML config {absolute_path:?}"))
    }

    /// Get all entries
    pub fn get_entries(&self) -> HashMap<String, Entry> {
        self.entries
            .iter()
            .map(|(name, entry)| (name.clone(), entry.clone()))
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
            .map(|(name, entry)| (name.clone(), entry.clone()))
            .collect()
    }

    /// Get a specific command configuration
    pub fn get_entry(&self, command: &str) -> Option<Entry> {
        self.entries.get(command).cloned()
    }

    /// Get an entry with constrains
    pub fn get_entry_with<F>(&self, name: &str, predicate: F) -> Option<Entry>
    where
        F: Fn(&Entry) -> bool,
    {
        self.entries
            .get(name)
            .filter(|entry| predicate(entry))
            .cloned()
    }

    /// Get all command entries (filtering by type: command)
    pub fn get_commands(&self) -> HashMap<String, Entry> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.entry_type == EntryType::Command)
            .map(|(name, entry)| (name.clone(), entry.clone()))
            .collect()
    }

    /// Get a specific command configuration
    pub fn get_command(&self, name: &str) -> Option<Entry> {
        self.entries
            .get(name)
            .filter(|entry| entry.entry_type == EntryType::Command)
            .cloned()
    }

    /// Get all model entries (filtering by type: command)
    pub fn get_models(&self) -> HashMap<String, Entry> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.entry_type == EntryType::Model)
            .map(|(name, entry)| (name.clone(), entry.clone()))
            .collect()
    }

    /// Get a model entry by name
    fn get_model(&self, name: &str) -> Option<Entry> {
        self.entries
            .get(name)
            .filter(|entry| entry.entry_type == EntryType::Model)
            .cloned()
    }

    /// Merge command config with its models (if includes is set)
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
        let cmd_setenv_if_unset = cmd_config.setenv_if_unset.clone();
        let cmd_setenv = cmd_config.setenv.clone();
        let cmd_unsetenv = cmd_config.unsetenv.clone();
        let cmd_cap = cmd_config.cap.clone();

        let mut result = Entry {
            entry_type: cmd_config.entry_type.clone(),
            enabled: cmd_config.enabled,
            override_parent: cmd_config.override_parent,
            includes: vec![], // Clear includes after processing
            share: HashSet::new(),
            bind: HashMap::new(),
            ro_bind: HashMap::new(),
            dev_bind: HashMap::new(),
            bind_try: HashMap::new(),
            ro_bind_try: HashMap::new(),
            dev_bind_try: HashMap::new(),
            tmpfs: HashSet::new(),
            chdir: cmd_config.chdir.clone(),
            die_with_parent: cmd_config.die_with_parent,
            new_session: cmd_config.new_session,
            cap: HashSet::new(),
            setenv_if_unset: HashMap::new(),
            setenv: HashMap::new(),
            unsetenv: HashSet::new(),
            alias: cmd_config.alias.clone(),
            args: cmd_config.args.clone(),
        };

        // Iterate over each model in the includes list
        for model_name in &cmd_config.includes {
            if let Some(template) = self.get_model(model_name) {
                // Later models replace prior mount destinations and union set values.
                result.share.extend(template.share.clone());
                result.bind.extend(template.bind.clone());
                result.ro_bind.extend(template.ro_bind.clone());
                result.dev_bind.extend(template.dev_bind.clone());
                result.bind_try.extend(template.bind_try.clone());
                result.ro_bind_try.extend(template.ro_bind_try.clone());
                result.dev_bind_try.extend(template.dev_bind_try.clone());
                result.tmpfs.extend(template.tmpfs.clone());
                result.unsetenv.extend(template.unsetenv.clone());
                result.cap.extend(template.cap.clone());

                // Later templates override earlier values in each environment map.
                result
                    .setenv_if_unset
                    .extend(template.setenv_if_unset.clone());
                result.setenv.extend(template.setenv.clone());
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
        result.unsetenv.extend(cmd_unsetenv);
        result.cap.extend(cmd_cap);
        result.setenv_if_unset.extend(cmd_setenv_if_unset);
        result.setenv.extend(cmd_setenv);
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

    fn mounts(entries: &[(&str, &str)]) -> HashMap<String, String> {
        let base_dir = std::env::current_dir().unwrap();
        entries
            .iter()
            .map(|(src, dst)| {
                (
                    resolve_path(dst, &base_dir, "test", "bind").unwrap(),
                    resolve_path(src, &base_dir, "test", "bind").unwrap(),
                )
            })
            .collect()
    }

    fn strings(entries: &[&str]) -> HashSet<String> {
        entries.iter().map(|entry| entry.to_string()).collect()
    }

    #[test]
    fn test_parse_basic_config() {
        let config = Config::from_yaml(indoc! {"
            node:
              enabled: true
              share:
                - user
                - network
              bind:
                - [~/.npm, ~/.npm]
        "})
        .unwrap();
        let commands = config.get_commands();
        assert_eq!(commands.len(), 1);
        assert!(commands.contains_key("node"));

        let node_cmd = commands.get("node").unwrap();
        assert!(node_cmd.enabled);
        assert_eq!(node_cmd.share, strings(&["user", "network"]));
        assert_eq!(node_cmd.bind, mounts(&[("~/.npm", "~/.npm")]));
    }

    #[test]
    fn test_bind_array_uses_destination_as_key_and_last_value_wins() {
        let config = Config::from_yaml(indoc! {"
            node:
              bind:
                - [/project-a, /workspace]
                - [/project-b, /workspace]
                - /tmp
        "})
        .unwrap();

        let node = config.get_command("node").unwrap();
        assert_eq!(node.bind.len(), 2);
        assert_eq!(node.bind.get("/workspace"), Some(&"/project-b".to_string()));
        assert_eq!(node.bind.get("/tmp"), Some(&"/tmp".to_string()));
    }

    #[test]
    fn test_bind_serialization_preserves_array_syntax() {
        let config = Config::from_yaml(indoc! {"
            node:
              bind:
                - /tmp
                - [/project, /workspace]
        "})
        .unwrap();

        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("bind:"));
        assert!(yaml.contains("- /tmp"));
        assert!(yaml.contains("- - /project\n    - /workspace"));
    }

    #[test]
    fn test_file_parsing_resolves_paths_before_mount_keying() {
        let directory = tempfile::tempdir().unwrap();
        let config_path = directory.path().join("config.yaml");
        fs::write(
            &config_path,
            format!(
                "node:\n  bind:\n    - [/first, .]\n    - [/second, {}]\n  tmpfs:\n    - ./tmp\n  chdir: $HOME\n",
                directory.path().display()
            ),
        )
        .unwrap();

        let config = Config::from_file(&config_path).unwrap();
        let node = config.get_command("node").unwrap();
        let directory = directory.path().to_string_lossy().into_owned();
        let home = std::env::var("HOME").unwrap();

        assert_eq!(node.bind.len(), 1);
        assert_eq!(node.bind.get(&directory), Some(&"/second".to_string()));
        assert!(node.tmpfs.contains(&format!("{directory}/tmp")));
        assert_eq!(node.chdir, Some(home));
        assert!(node.bind.keys().all(|path| Path::new(path).is_absolute()));
        assert!(node.bind.values().all(|path| Path::new(path).is_absolute()));
        assert!(node.tmpfs.iter().all(|path| Path::new(path).is_absolute()));
        assert!(
            node.chdir
                .as_ref()
                .is_some_and(|path| Path::new(path).is_absolute())
        );
    }

    #[test]
    fn test_relative_config_filename_resolves_model_paths_absolutely() {
        let current_dir = std::env::current_dir().unwrap();
        let mut config_file = NamedTempFile::new_in(&current_dir).unwrap();
        config_file.write_all(b"node:\n  bind:\n    - .\n").unwrap();
        let relative_path = config_file.path().strip_prefix(&current_dir).unwrap();

        let config = Config::from_file(relative_path).unwrap();
        let node = config.get_command("node").unwrap();
        let current_dir = current_dir.to_string_lossy().into_owned();

        assert_eq!(node.bind.get(&current_dir), Some(&current_dir));
    }

    #[test]
    fn test_unset_path_variable_is_rejected() {
        let error = Config::from_yaml(
            "node:\n  bind:\n    - $SHELD_PATH_VARIABLE_THAT_MUST_NOT_EXIST_0A46D88E\n",
        )
        .unwrap_err();

        let message = format!("{error:#}");
        assert!(message.contains("SHELD_PATH_VARIABLE_THAT_MUST_NOT_EXIST_0A46D88E"));
        assert!(message.contains("bind"));
        assert!(message.contains("node"));
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
              includes: base
              bind:
                - [~/.npm, ~/.npm]
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        assert_eq!(node_cmd.includes, vec!["base"]);
        assert_eq!(node_cmd.bind, mounts(&[("~/.npm", "~/.npm")]));
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
              includes: base
              bind:
                - [~/.npm, ~/.npm]
        "})
        .unwrap();
        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_base(node_cmd);

        // Should have both base and command-specific settings
        assert_eq!(merged.share, strings(&["user"]));

        assert_eq!(merged.ro_bind, mounts(&[("/usr", "/usr")]));
        assert_eq!(merged.bind, mounts(&[("~/.npm", "~/.npm")]));
    }

    #[test]
    fn test_merge_without_includes() {
        let config = Config::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user

            node:
              bind:
                - [~/.npm, ~/.npm]
        "})
        .unwrap();
        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_base(node_cmd.clone());

        // Should not merge base since includes is not set
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
    fn test_environment_variables() {
        let config = Config::from_yaml(indoc! {"
            node:
              setenv_if_unset:
                NODE_ENV: production
              setenv:
                PATH: /custom/path
              unsetenv:
                - DEBUG
        "})
        .unwrap();
        let node_cmd = config.get_command("node").unwrap();

        assert_eq!(node_cmd.setenv_if_unset.len(), 1);
        assert_eq!(
            node_cmd.setenv_if_unset.get("NODE_ENV"),
            Some(&"production".to_string())
        );
        assert_eq!(
            node_cmd.setenv.get("PATH"),
            Some(&"/custom/path".to_string())
        );
        assert_eq!(node_cmd.unsetenv, strings(&["DEBUG"]));
    }

    #[test]
    fn test_environment_legacy_and_unknown_fields_are_rejected() {
        for field in ["env", "unset_env", "setenv_if_unsettt"] {
            let yaml = format!("node:\n  {field}: value\n");
            let error = Config::from_yaml(&yaml).unwrap_err();

            assert!(format!("{error:#}").contains(field));
        }
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
        assert_eq!(node_cmd.tmpfs, strings(&["/tmp", "/var/tmp"]));
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
        assert_eq!(
            node_cmd.dev_bind,
            mounts(&[("/dev/null", "/dev/null"), ("/dev/random", "/dev/random")])
        );
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
              includes: minimal
              bind:
                - [~/.npm, ~/.npm]
            python:
              includes: strict
              bind:
                - [~/.local, ~/.local]
        "})
        .unwrap();

        // Verify we have 2 commands
        let commands = config.get_commands();
        assert_eq!(commands.len(), 2);

        // Test node with minimal template
        let node_cmd = config.get_command("node").unwrap();
        assert_eq!(node_cmd.includes, vec!["minimal"]);
        let merged_node = config.merge_with_template(node_cmd);
        assert_eq!(merged_node.share, strings(&["user", "network"]));
        assert_eq!(merged_node.bind, mounts(&[("~/.npm", "~/.npm")]));

        // Test python with strict template
        let python_cmd = config.get_command("python").unwrap();
        assert_eq!(python_cmd.includes, vec!["strict"]);
        let merged_python = config.merge_with_template(python_cmd);
        assert_eq!(merged_python.share, strings(&["user"]));
        assert_eq!(merged_python.ro_bind, mounts(&[("/usr", "/usr")]));
        assert_eq!(merged_python.bind, mounts(&[("~/.local", "~/.local")]));
    }

    #[test]
    fn test_nonexistent_template() {
        let config = Config::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user

            node:
              includes: nonexistent
              bind:
                - [~/.npm, ~/.npm]
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
              includes: base
              bind:
                - [~/.npm, ~/.npm]

            python:
              enabled: false
              includes: base
              bind:
                - [~/.local, ~/.local]

            rust:
              enabled: true
              includes: base
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
        let with_network = config.get_entries_with(|e| e.share.contains("network"));
        assert_eq!(with_network.len(), 1);
        assert!(with_network.contains_key("rust"));

        // Filter entries that include base
        let includes_base = config.get_entries_with(|e| e.includes.contains(&"base".to_string()));
        assert_eq!(includes_base.len(), 3);

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
              includes: base
              share:
                - network
              bind:
                - [~/.npm, ~/.npm]

            python:
              enabled: false
              includes: base
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
        let node_network = config.get_entry_with("node", |e| e.share.contains("network"));
        assert!(node_network.is_some());

        let python_network = config.get_entry_with("python", |e| e.share.contains("network"));
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

        let no_network = config.get_entries_with(|e| e.share.contains("network"));
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
        assert_eq!(node_cmd.share, strings(&["network"]));
    }

    #[test]
    fn test_merge_local_command_includes_user_model() {
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
              includes: base
              bind:
                - [~/.npm, ~/.npm]
        "})
        .unwrap();

        let merged = Config::merge(user_config, local_config);
        let node_cmd = merged.get_command("node").unwrap();
        let with_template = merged.merge_with_template(node_cmd);

        // Should inherit from user's base model
        assert_eq!(with_template.share, strings(&["user"]));
        assert_eq!(with_template.ro_bind, mounts(&[("/usr", "/usr")]));
        assert_eq!(with_template.bind, mounts(&[("~/.npm", "~/.npm")]));
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
        assert_eq!(base_model.share, strings(&["network"]));
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
        assert_eq!(node_cmd.share, strings(&["user"]));
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
        assert!(!node_cmd.override_parent);
    }

    #[test]
    fn test_override_true_replaces_parent() {
        let parent_config = Config::from_yaml(indoc! {"
            node:
              share:
                - user
                - pid
              bind:
                - [/usr, /usr]
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
        assert_eq!(node_cmd.share, strings(&["network"]));
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

        // Sets are merged by union.
        assert_eq!(node_cmd.share, strings(&["user", "pid", "network"]));
    }

    #[test]
    fn test_deep_merge_child_mount_replaces_parent_destination() {
        let parent = Config::from_yaml(indoc! {"
            node:
              bind:
                - [/parent, /workspace]
        "})
        .unwrap();
        let child = Config::from_yaml(indoc! {"
            node:
              bind:
                - [/child, /workspace]
        "})
        .unwrap();

        let merged = Config::merge(parent, child);
        let node = merged.get_command("node").unwrap();
        assert_eq!(node.bind, mounts(&[("/child", "/workspace")]));
    }

    #[test]
    fn test_override_false_merges_environment_maps_and_unsetenv() {
        let parent_config = Config::from_yaml(indoc! {"
            node:
              setenv_if_unset:
                FROM: parent
                KEEP: this
              setenv:
                PATH: /parent/path
              unsetenv:
                - TOKEN
                - SECRET
        "})
        .unwrap();

        let child_config = Config::from_yaml(indoc! {"
            node:
              setenv_if_unset:
                FROM: child
                NEW: value
              setenv:
                PATH: /child/path
              unsetenv:
                - SECRET
                - API_KEY
        "})
        .unwrap();

        let merged = Config::merge(parent_config, child_config);
        let node_cmd = merged.get_command("node").unwrap();

        assert_eq!(
            node_cmd.setenv_if_unset.get("FROM"),
            Some(&"child".to_string())
        );
        assert_eq!(
            node_cmd.setenv_if_unset.get("KEEP"),
            Some(&"this".to_string())
        );
        assert_eq!(
            node_cmd.setenv_if_unset.get("NEW"),
            Some(&"value".to_string())
        );
        assert_eq!(
            node_cmd.setenv.get("PATH"),
            Some(&"/child/path".to_string())
        );
        assert_eq!(node_cmd.unsetenv, strings(&["TOKEN", "SECRET", "API_KEY"]));
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
        assert_eq!(node_cmd.share, strings(&["user", "network"]));
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
        assert_eq!(node_cmd.share, strings(&["user"]));
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
        assert_eq!(node_cmd.share, strings(&["user"]));
    }

    #[test]
    fn test_includes_single_string_syntax() {
        let config = Config::from_yaml(indoc! {"
            base:
              type: model
              share:
                - user

            node:
              includes: base
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        assert_eq!(node_cmd.includes, vec!["base"]);
    }

    #[test]
    fn test_includes_list_syntax_multiple_models() {
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
              includes: [base, network]
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        assert_eq!(node_cmd.includes, vec!["base", "network"]);
    }

    #[test]
    fn test_includes_models_applied_in_order() {
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
              includes: [base, network]
              bind:
                - [~/.npm, ~/.npm]
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_template(node_cmd);

        // Should have shares from both models
        assert!(merged.share.contains("user"));
        assert!(merged.share.contains("network"));

        // Should have ro_bind from both models.
        assert_eq!(merged.ro_bind.get("/usr"), Some(&"/usr".to_string()));
        assert_eq!(
            merged.ro_bind.get("/etc/resolv.conf"),
            Some(&"/etc/resolv.conf".to_string())
        );

        // Should have bind from command itself
        let npm_path =
            resolve_path("~/.npm", &std::env::current_dir().unwrap(), "test", "bind").unwrap();
        assert_eq!(merged.bind.get(&npm_path), Some(&npm_path));
    }

    #[test]
    fn test_includes_later_model_overrides_earlier_setenv() {
        let config = Config::from_yaml(indoc! {"
            base:
              type: model
              setenv:
                KEY: base_value
                OTHER: keep_this

            override:
              type: model
              setenv:
                KEY: override_value

            node:
              includes: [base, override]
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_template(node_cmd);

        assert_eq!(
            merged.setenv.get("KEY"),
            Some(&"override_value".to_string())
        );
        assert_eq!(merged.setenv.get("OTHER"), Some(&"keep_this".to_string()));
    }

    #[test]
    fn test_includes_later_model_and_command_override_mount_destination() {
        let config = Config::from_yaml(indoc! {"
            base:
              type: model
              bind:
                - [/base, /workspace]

            override:
              type: model
              bind:
                - [/override, /workspace]

            model_only:
              includes: [base, override]

            command:
              includes: [base, override]
              bind:
                - [/command, /workspace]
        "})
        .unwrap();

        let model_only = config.get_command("model_only").unwrap();
        let merged_model_only = config.merge_with_template(model_only);
        assert_eq!(
            merged_model_only.bind.get("/workspace"),
            Some(&"/override".to_string())
        );

        let command = config.get_command("command").unwrap();
        let merged_command = config.merge_with_template(command);
        assert_eq!(
            merged_command.bind.get("/workspace"),
            Some(&"/command".to_string())
        );
    }

    #[test]
    fn test_includes_entry_settings_override_all_models() {
        let config = Config::from_yaml(indoc! {"
            base:
              type: model
              setenv:
                KEY: base_value

            network:
              type: model
              setenv:
                KEY: network_value

            node:
              includes: [base, network]
              setenv:
                KEY: command_value
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_template(node_cmd);

        assert_eq!(merged.setenv.get("KEY"), Some(&"command_value".to_string()));
    }

    #[test]
    fn test_includes_skip_nonexistent_model() {
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
              includes: [base, nonexistent, network]
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_template(node_cmd);

        // Should apply base and network, skip nonexistent
        assert!(merged.share.contains("user"));
        assert!(merged.share.contains("network"));
    }

    #[test]
    fn test_includes_all_models_nonexistent() {
        let config = Config::from_yaml(indoc! {"
            node:
              includes: [foo, bar]
              share:
                - user
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        let merged = config.merge_with_template(node_cmd);

        // Should just have command's own settings
        assert_eq!(merged.share, strings(&["user"]));
    }

    #[test]
    fn test_includes_empty_list() {
        let config = Config::from_yaml(indoc! {"
            node:
              includes: []
              share:
                - user
        "})
        .unwrap();

        let node_cmd = config.get_command("node").unwrap();
        assert_eq!(node_cmd.includes, Vec::<String>::new());

        let merged = config.merge_with_template(node_cmd);
        assert_eq!(merged.share, strings(&["user"]));
    }

    #[test]
    fn test_alias_not_merged_from_parent() {
        // Parent has alias, child does not — child should not inherit it
        let parent = Config::from_yaml(indoc! {"
            chromium-dev:
              alias: chromium
              share:
                - user
        "})
        .unwrap();

        let child = Config::from_yaml(indoc! {"
            chromium-dev:
              share:
                - network
        "})
        .unwrap();

        let merged = Config::merge(parent, child);
        let cmd = merged.get_command("chromium-dev").unwrap();
        assert_eq!(cmd.alias, None);
    }

    #[test]
    fn test_alias_kept_when_set_on_child() {
        // Child sets alias — it should be present after merge
        let parent = Config::from_yaml(indoc! {"
            chromium-dev:
              share:
                - user
        "})
        .unwrap();

        let child = Config::from_yaml(indoc! {"
            chromium-dev:
              alias: chromium
              share:
                - network
        "})
        .unwrap();

        let merged = Config::merge(parent, child);
        let cmd = merged.get_command("chromium-dev").unwrap();
        assert_eq!(cmd.alias, Some("chromium".to_string()));
    }

    #[test]
    fn test_args_not_merged_from_parent() {
        let parent = Config::from_yaml(indoc! {"
            chromium-dev:
              args:
                - --no-sandbox
              share:
                - user
        "})
        .unwrap();

        let child = Config::from_yaml(indoc! {"
            chromium-dev:
              share:
                - network
        "})
        .unwrap();

        let merged = Config::merge(parent, child);
        let cmd = merged.get_command("chromium-dev").unwrap();
        assert_eq!(cmd.args, Vec::<String>::new());
    }

    #[test]
    fn test_args_kept_when_set_on_child() {
        let parent = Config::from_yaml(indoc! {"
            chromium-dev:
              share:
                - user
        "})
        .unwrap();

        let child = Config::from_yaml(indoc! {"
            chromium-dev:
              args:
                - --no-sandbox
              share:
                - network
        "})
        .unwrap();

        let merged = Config::merge(parent, child);
        let cmd = merged.get_command("chromium-dev").unwrap();
        assert_eq!(cmd.args, vec!["--no-sandbox"]);
    }
}
