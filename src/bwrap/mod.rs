use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Entry;

const NAMESPACES: [&str; 6] = ["user", "pid", "network", "ipc", "uts", "cgroup"];

pub struct WrappedCommandBuilder {
    config: Entry,
    config_dir: Option<PathBuf>,
}

impl WrappedCommandBuilder {
    pub fn new(config: Entry, config_dir: Option<PathBuf>) -> Self {
        Self { config, config_dir }
    }

    fn resolve_path(&self, raw: &str) -> String {
        let expanded = shellexpand::full(raw).unwrap_or_else(|_| raw.into());
        let path = Path::new(expanded.as_ref());
        if path.is_relative()
            && let Some(dir) = &self.config_dir
        {
            let joined = dir.join(path);
            let normalized: PathBuf = joined.components().collect();
            return normalized.to_string_lossy().into_owned();
        }
        expanded.into_owned()
    }

    /// Build the bwrap command arguments
    pub fn build_args(&self) -> Vec<String> {
        self.build_args_with_environment(|key| std::env::var_os(key).is_some())
    }

    fn build_args_with_environment(
        &self,
        inherited_environment_contains: impl Fn(&str) -> bool,
    ) -> Vec<String> {
        let mut args = Vec::new();

        // Add boolean flags first
        if self.config.die_with_parent {
            args.push("--die-with-parent".to_string());
        }

        if self.config.new_session {
            args.push("--new-session".to_string());
        }

        // Determine which namespaces to unshare (all by default, except those in share)
        let shared_namespaces: std::collections::HashSet<&str> =
            self.config.share.iter().map(|s| s.as_str()).collect();

        // Unshare all namespaces except those explicitly shared
        for namespace in &NAMESPACES {
            if !shared_namespaces.contains(namespace) {
                match *namespace {
                    "network" => args.push("--unshare-net".to_string()),
                    "pid" => args.push("--unshare-pid".to_string()),
                    "ipc" => args.push("--unshare-ipc".to_string()),
                    "uts" => args.push("--unshare-uts".to_string()),
                    "user" => args.push("--unshare-user".to_string()),
                    "cgroup" => args.push("--unshare-cgroup".to_string()),
                    _ => {}
                }
            }
        }

        // Handle custom bind mounts
        for (src, dst) in &self.config.bind {
            let src_expanded = self.resolve_path(src);
            let dst_expanded = self.resolve_path(dst);
            args.push("--bind".to_string());
            args.push(src_expanded);
            args.push(dst_expanded);
        }

        // Handle read-only binds
        for (src, dst) in &self.config.ro_bind {
            let src_expanded = self.resolve_path(src);
            let dst_expanded = self.resolve_path(dst);
            args.push("--ro-bind".to_string());
            args.push(src_expanded);
            args.push(dst_expanded);
        }

        // Handle device binds
        for (src, dst) in &self.config.dev_bind {
            let src_expanded = self.resolve_path(src);
            let dst_expanded = self.resolve_path(dst);
            args.push("--dev-bind".to_string());
            args.push(src_expanded);
            args.push(dst_expanded);
        }

        // Handle bind-try
        for (src, dst) in &self.config.bind_try {
            let src_expanded = self.resolve_path(src);
            let dst_expanded = self.resolve_path(dst);
            args.push("--bind-try".to_string());
            args.push(src_expanded);
            args.push(dst_expanded);
        }

        // Handle read-only bind-try
        for (src, dst) in &self.config.ro_bind_try {
            let src_expanded = self.resolve_path(src);
            let dst_expanded = self.resolve_path(dst);
            args.push("--ro-bind-try".to_string());
            args.push(src_expanded);
            args.push(dst_expanded);
        }

        // Handle device bind-try
        for (src, dst) in &self.config.dev_bind_try {
            let src_expanded = self.resolve_path(src);
            let dst_expanded = self.resolve_path(dst);
            args.push("--dev-bind-try".to_string());
            args.push(src_expanded);
            args.push(dst_expanded);
        }

        // Handle tmpfs
        for tmpfs in &self.config.tmpfs {
            args.push("--tmpfs".to_string());
            args.push(tmpfs.clone());
        }

        // Handle chdir
        if let Some(chdir) = &self.config.chdir {
            let expanded = self.resolve_path(chdir);
            args.push("--chdir".to_string());
            args.push(expanded);
        }

        // Handle cap
        for cap in &self.config.cap {
            args.push("--cap-add".to_string());
            args.push(cap.clone());
        }

        let unsetenv: std::collections::HashSet<&str> =
            self.config.unsetenv.iter().map(String::as_str).collect();

        // Defaults do not override inherited or forced values.
        for (key, value) in &self.config.setenv_if_unset {
            if unsetenv.contains(key.as_str())
                || self.config.setenv.contains_key(key)
                || inherited_environment_contains(key)
            {
                continue;
            }

            args.push("--setenv".to_string());
            args.push(key.clone());
            args.push(value.clone());
        }

        // Forced values override inherited and default values.
        for (key, value) in &self.config.setenv {
            if unsetenv.contains(key.as_str()) {
                continue;
            }

            args.push("--setenv".to_string());
            args.push(key.clone());
            args.push(value.clone());
        }

        // Unset values are emitted last so they override every other source.
        let mut emitted_unsetenv = std::collections::HashSet::new();
        for key in &self.config.unsetenv {
            if !emitted_unsetenv.insert(key.as_str()) {
                continue;
            }

            args.push("--unsetenv".to_string());
            args.push(key.clone());
        }

        args
    }

    /// Execute a command with bwrap
    pub fn exec(&self, command: &str, command_args: &[String]) -> Result<i32> {
        let bwrap_args = self.build_args();

        let mut cmd = Command::new("bwrap");
        cmd.args(&bwrap_args);
        cmd.arg(command);
        cmd.args(&self.config.args);
        cmd.args(command_args);

        let status = cmd.status()?;
        Ok(status.code().unwrap_or(1))
    }

    /// Show the bwrap command that would be executed (dry-run)
    pub fn show(&self, command: &str, command_args: &[String]) -> String {
        let bwrap_args = self.build_args();

        let mut parts = vec!["bwrap".to_string()];
        parts.extend(bwrap_args);
        parts.push(command.to_string());
        parts.extend(self.config.args.iter().cloned());
        parts.extend(command_args.iter().cloned());

        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, EntryType};

    use super::*;
    use std::collections::HashMap;

    fn create_test_config() -> Entry {
        Entry {
            entry_type: EntryType::Command,
            enabled: true,
            override_parent: false,
            includes: vec![],
            share: vec![],
            bind: vec![],
            ro_bind: vec![],
            dev_bind: vec![],
            bind_try: vec![],
            ro_bind_try: vec![],
            dev_bind_try: vec![],
            tmpfs: vec![],
            chdir: None,
            die_with_parent: false,
            new_session: false,
            cap: vec![],
            setenv_if_unset: HashMap::new(),
            setenv: HashMap::new(),
            unsetenv: vec![],
            alias: None,
            args: vec![],
        }
    }

    #[test]
    fn test_build_args_unshare_all_default() {
        let config = create_test_config();
        // Empty config = all namespaces unshared by default

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        assert!(args.contains(&"--unshare-net".to_string()));
        assert!(args.contains(&"--unshare-pid".to_string()));
        assert!(args.contains(&"--unshare-ipc".to_string()));
        assert!(args.contains(&"--unshare-uts".to_string()));
        assert!(args.contains(&"--unshare-user".to_string()));
        assert!(args.contains(&"--unshare-cgroup".to_string()));
    }

    #[test]
    fn test_build_args_share() {
        let mut config = create_test_config();
        // share now controls namespace sharing, not filesystem paths
        config.share = vec!["network".to_string(), "user".to_string()];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        // Network and user should NOT be unshared
        assert!(!args.contains(&"--unshare-net".to_string()));
        assert!(!args.contains(&"--unshare-user".to_string()));

        // But other namespaces should be unshared
        assert!(args.contains(&"--unshare-pid".to_string()));
        assert!(args.contains(&"--unshare-ipc".to_string()));
        assert!(args.contains(&"--unshare-uts".to_string()));
        assert!(args.contains(&"--unshare-cgroup".to_string()));
    }

    #[test]
    fn test_build_args_bind() {
        let mut config = create_test_config();
        config.bind = vec![("/src".to_string(), "/dest".to_string())];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        let bind_idx = args.iter().position(|x| x == "--bind").unwrap();
        assert_eq!(args[bind_idx + 1], "/src");
        assert_eq!(args[bind_idx + 2], "/dest");
    }

    #[test]
    fn test_build_args_ro_bind() {
        let mut config = create_test_config();
        config.ro_bind = vec![("/usr".to_string(), "/usr".to_string())];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        assert!(args.contains(&"--ro-bind".to_string()));
        assert!(args.contains(&"/usr".to_string()));
    }

    #[test]
    fn test_build_args_dev_bind() {
        let mut config = create_test_config();
        config.dev_bind = vec![("/dev/null".to_string(), "/dev/null".to_string())];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        assert!(args.contains(&"--dev-bind".to_string()));
        assert!(args.contains(&"/dev/null".to_string()));
    }

    #[test]
    fn test_build_args_tmpfs() {
        let mut config = create_test_config();
        config.tmpfs = vec!["/tmp".to_string(), "/var/tmp".to_string()];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        assert!(args.contains(&"--tmpfs".to_string()));
        assert!(args.contains(&"/tmp".to_string()));
        assert!(args.contains(&"/var/tmp".to_string()));
    }

    #[test]
    fn test_build_args_setenv() {
        let mut config = create_test_config();
        config
            .setenv
            .insert("NODE_ENV".to_string(), "production".to_string());
        config
            .setenv
            .insert("DEBUG".to_string(), "true".to_string());

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        let setenv_count = args.iter().filter(|x| *x == "--setenv").count();
        assert_eq!(setenv_count, 2);
        assert!(args.contains(&"NODE_ENV".to_string()));
        assert!(args.contains(&"production".to_string()));
    }

    fn contains_setenv(args: &[String], key: &str, value: &str) -> bool {
        args.windows(3)
            .any(|window| window == ["--setenv", key, value])
    }

    #[test]
    fn test_setenv_if_unset_is_emitted_when_not_inherited() {
        let mut config = create_test_config();
        config.setenv_if_unset.insert(
            "DOCKER_HOST".to_string(),
            "unix:///default.sock".to_string(),
        );

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args_with_environment(|_| false);

        assert!(contains_setenv(
            &args,
            "DOCKER_HOST",
            "unix:///default.sock"
        ));
    }

    #[test]
    fn test_setenv_if_unset_does_not_override_inherited_value() {
        let mut config = create_test_config();
        config.setenv_if_unset.insert(
            "DOCKER_HOST".to_string(),
            "unix:///default.sock".to_string(),
        );

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args_with_environment(|key| key == "DOCKER_HOST");

        assert!(!contains_setenv(
            &args,
            "DOCKER_HOST",
            "unix:///default.sock"
        ));
    }

    #[test]
    fn test_setenv_if_unset_treats_empty_inherited_value_as_present() {
        const KEY: &str = "SHELD_TEST_EMPTY_INHERITED_VALUE";

        let mut config = create_test_config();
        config
            .setenv_if_unset
            .insert(KEY.to_string(), "unix:///default.sock".to_string());

        let builder = WrappedCommandBuilder::new(config, None);
        let previous_value = std::env::var_os(KEY);
        unsafe { std::env::set_var(KEY, "") };
        let args = builder.build_args();

        match previous_value {
            Some(value) => unsafe { std::env::set_var(KEY, value) },
            None => unsafe { std::env::remove_var(KEY) },
        }

        assert!(!contains_setenv(&args, KEY, "unix:///default.sock"));
    }

    #[test]
    fn test_setenv_overrides_inherited_value() {
        let mut config = create_test_config();
        config
            .setenv
            .insert("PATH".to_string(), "/controlled/path".to_string());

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args_with_environment(|key| key == "PATH");

        assert!(contains_setenv(&args, "PATH", "/controlled/path"));
    }

    #[test]
    fn test_unsetenv_overrides_default_and_forced_values() {
        let mut config = create_test_config();
        config
            .setenv_if_unset
            .insert("SECRET_TOKEN".to_string(), "default-secret".to_string());
        config
            .setenv
            .insert("SECRET_TOKEN".to_string(), "forced-secret".to_string());
        config.unsetenv = vec!["SECRET_TOKEN".to_string()];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args_with_environment(|key| key == "SECRET_TOKEN");

        assert!(!contains_setenv(&args, "SECRET_TOKEN", "default-secret"));
        assert!(!contains_setenv(&args, "SECRET_TOKEN", "forced-secret"));
        assert!(
            args.windows(2)
                .any(|window| window == ["--unsetenv", "SECRET_TOKEN"])
        );
    }

    #[test]
    fn test_build_args_unsetenv() {
        let mut config = create_test_config();
        config.unsetenv = vec![
            "DEBUG".to_string(),
            "VERBOSE".to_string(),
            "DEBUG".to_string(),
        ];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        assert!(args.contains(&"--unsetenv".to_string()));
        assert!(args.contains(&"DEBUG".to_string()));
        assert!(args.contains(&"VERBOSE".to_string()));
        assert_eq!(
            args.windows(2)
                .filter(|window| *window == ["--unsetenv", "DEBUG"])
                .count(),
            1
        );
    }

    #[test]
    fn test_build_args_with_merged_model_environment() {
        let config = Config::from_yaml(
            r#"
                base:
                  type: model
                  setenv_if_unset:
                    DEFAULT: model-default
                  setenv:
                    FORCE: model-value
                  unsetenv:
                    - SECRET

                command:
                  includes: base
                  setenv:
                    FORCE: command-value
                  unsetenv:
                    - SECRET
            "#,
        )
        .unwrap();
        let command = config.get_command("command").unwrap();
        let merged = config.merge_with_template(command);
        let builder = WrappedCommandBuilder::new(merged, None);
        let args = builder.build_args_with_environment(|key| key == "DEFAULT");

        assert!(!contains_setenv(&args, "DEFAULT", "model-default"));
        assert!(contains_setenv(&args, "FORCE", "command-value"));
        assert!(!contains_setenv(&args, "FORCE", "model-value"));
        assert_eq!(
            args.windows(2)
                .filter(|window| *window == ["--unsetenv", "SECRET"])
                .count(),
            1
        );
    }

    #[test]
    fn test_build_args_combined() {
        let mut config = create_test_config();
        config.share = vec!["user".to_string()]; // Share only user namespace
        config.ro_bind = vec![("/usr".to_string(), "/usr".to_string())];
        config
            .setenv
            .insert("TEST".to_string(), "value".to_string());

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        // Check all types are present
        assert!(args.contains(&"--unshare-net".to_string()));
        assert!(!args.contains(&"--unshare-user".to_string())); // user is shared
        assert!(args.contains(&"--ro-bind".to_string()));
        assert!(args.contains(&"--setenv".to_string()));
    }

    #[test]
    fn test_show_command() {
        let mut config = create_test_config();
        config.share = vec!["user".to_string()]; // Share user, unshare rest

        let builder = WrappedCommandBuilder::new(config, None);
        let cmd = builder.show("node", &["script.js".to_string()]);

        assert!(cmd.starts_with("bwrap"));
        assert!(cmd.contains("--unshare-net"));
        assert!(cmd.contains("node"));
        assert!(cmd.contains("script.js"));
    }

    #[test]
    fn test_show_command_with_multiple_args() {
        let config = create_test_config();
        let builder = WrappedCommandBuilder::new(config, None);
        let cmd = builder.show(
            "git",
            &[
                "commit".to_string(),
                "-m".to_string(),
                "message".to_string(),
            ],
        );

        assert!(cmd.contains("git"));
        assert!(cmd.contains("commit"));
        assert!(cmd.contains("-m"));
        assert!(cmd.contains("message"));
    }

    #[test]
    fn test_empty_config() {
        let config = create_test_config();
        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        // Empty config should unshare all namespaces by default
        assert!(args.contains(&"--unshare-net".to_string()));
        assert!(args.contains(&"--unshare-pid".to_string()));
        assert!(args.contains(&"--unshare-ipc".to_string()));
        assert!(args.contains(&"--unshare-uts".to_string()));
        assert!(args.contains(&"--unshare-user".to_string()));
        assert!(args.contains(&"--unshare-cgroup".to_string()));
    }

    #[test]
    fn test_bind_with_tilde() {
        let mut config = create_test_config();
        config.bind = vec![("~/.config".to_string(), "~/.config".to_string())];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        // shellexpand should expand ~ to home directory
        let bind_idx = args.iter().position(|x| x == "--bind").unwrap();
        // The expanded path should not contain ~
        assert!(!args[bind_idx + 1].contains('~'));
    }

    #[test]
    fn test_unshare_all_by_default() {
        let config = create_test_config();
        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        // All namespaces should be unshared by default
        assert!(args.contains(&"--unshare-net".to_string()));
        assert!(args.contains(&"--unshare-pid".to_string()));
        assert!(args.contains(&"--unshare-ipc".to_string()));
        assert!(args.contains(&"--unshare-uts".to_string()));
        assert!(args.contains(&"--unshare-user".to_string()));
        assert!(args.contains(&"--unshare-cgroup".to_string()));
    }

    #[test]
    fn test_share_specific_namespaces() {
        let mut config = create_test_config();
        config.share = vec!["user".to_string(), "network".to_string()];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        // User and network should NOT be unshared (they are shared)
        assert!(!args.contains(&"--unshare-user".to_string()));
        assert!(!args.contains(&"--unshare-net".to_string()));

        // All other namespaces should still be unshared
        assert!(args.contains(&"--unshare-pid".to_string()));
        assert!(args.contains(&"--unshare-ipc".to_string()));
        assert!(args.contains(&"--unshare-uts".to_string()));
        assert!(args.contains(&"--unshare-cgroup".to_string()));
    }

    #[test]
    fn test_share_all_namespaces() {
        let mut config = create_test_config();
        config.share = vec![
            "user".to_string(),
            "pid".to_string(),
            "network".to_string(),
            "ipc".to_string(),
            "uts".to_string(),
            "cgroup".to_string(),
        ];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        // No namespaces should be unshared
        assert!(!args.contains(&"--unshare-user".to_string()));
        assert!(!args.contains(&"--unshare-pid".to_string()));
        assert!(!args.contains(&"--unshare-net".to_string()));
        assert!(!args.contains(&"--unshare-ipc".to_string()));
        assert!(!args.contains(&"--unshare-uts".to_string()));
        assert!(!args.contains(&"--unshare-cgroup".to_string()));
    }

    #[test]
    fn test_bind_try() {
        let mut config = create_test_config();
        config.bind_try = vec![("~/.cache".to_string(), "~/.cache".to_string())];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        let bind_try_idx = args.iter().position(|x| x == "--bind-try").unwrap();
        // Tilde should be expanded
        assert!(!args[bind_try_idx + 1].contains('~'));
        assert!(!args[bind_try_idx + 2].contains('~'));
    }

    #[test]
    fn test_ro_bind_try() {
        let mut config = create_test_config();
        config.ro_bind_try = vec![("/usr".to_string(), "/usr".to_string())];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        assert!(args.contains(&"--ro-bind-try".to_string()));
        assert!(args.contains(&"/usr".to_string()));
    }

    #[test]
    fn test_dev_bind_try() {
        let mut config = create_test_config();
        config.dev_bind_try = vec![("/dev/kvm".to_string(), "/dev/kvm".to_string())];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        assert!(args.contains(&"--dev-bind-try".to_string()));
        assert!(args.contains(&"/dev/kvm".to_string()));
    }

    #[test]
    fn test_chdir() {
        let mut config = create_test_config();
        config.chdir = Some("/workspace".to_string());

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        let chdir_idx = args.iter().position(|x| x == "--chdir").unwrap();
        assert_eq!(args[chdir_idx + 1], "/workspace");
    }

    #[test]
    fn test_chdir_with_tilde() {
        let mut config = create_test_config();
        config.chdir = Some("~/projects".to_string());

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        let chdir_idx = args.iter().position(|x| x == "--chdir").unwrap();
        // Tilde should be expanded
        assert!(!args[chdir_idx + 1].contains('~'));
    }

    #[test]
    fn test_chdir_none() {
        let config = create_test_config();
        // chdir is None by default

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        // Should not contain --chdir
        assert!(!args.contains(&"--chdir".to_string()));
    }

    #[test]
    fn test_die_with_parent_true() {
        let mut config = create_test_config();
        config.die_with_parent = true;

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        assert!(args.contains(&"--die-with-parent".to_string()));
    }

    #[test]
    fn test_die_with_parent_false() {
        let config = create_test_config();
        // die_with_parent is false by default

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        // Should not contain --die-with-parent
        assert!(!args.contains(&"--die-with-parent".to_string()));
    }

    #[test]
    fn test_new_session_true() {
        let mut config = create_test_config();
        config.new_session = true;

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        assert!(args.contains(&"--new-session".to_string()));
    }

    #[test]
    fn test_capabilities_single() {
        let mut config = create_test_config();
        config.cap = vec!["CAP_SYS_ADMIN".to_string()];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        let cap_add_idx = args.iter().position(|x| x == "--cap-add").unwrap();
        assert_eq!(args[cap_add_idx + 1], "CAP_SYS_ADMIN");
    }

    #[test]
    fn test_capabilities_multiple() {
        let mut config = create_test_config();
        config.cap = vec![
            "CAP_SYS_ADMIN".to_string(),
            "CAP_NET_ADMIN".to_string(),
            "CAP_SYS_TIME".to_string(),
        ];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        let cap_add_count = args.iter().filter(|x| *x == "--cap-add").count();
        assert_eq!(cap_add_count, 3);
        assert!(args.contains(&"CAP_SYS_ADMIN".to_string()));
        assert!(args.contains(&"CAP_NET_ADMIN".to_string()));
        assert!(args.contains(&"CAP_SYS_TIME".to_string()));
    }

    #[test]
    fn test_capabilities_empty() {
        let config = create_test_config();
        // capabilities is empty by default

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        // Should not contain --cap-add
        assert!(!args.contains(&"--cap-add".to_string()));
    }

    #[test]
    fn test_all_new_options_combined() {
        let mut config = create_test_config();
        config.bind_try = vec![("/tmp".to_string(), "/tmp".to_string())];
        config.ro_bind_try = vec![("/usr".to_string(), "/usr".to_string())];
        config.chdir = Some("/workspace".to_string());
        config.die_with_parent = true;
        config.new_session = true;
        config.cap = vec!["CAP_SYS_ADMIN".to_string()];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        // Check all new options are present
        assert!(args.contains(&"--die-with-parent".to_string()));
        assert!(args.contains(&"--new-session".to_string()));
        assert!(args.contains(&"--bind-try".to_string()));
        assert!(args.contains(&"--ro-bind-try".to_string()));
        assert!(args.contains(&"--chdir".to_string()));
        assert!(args.contains(&"--cap-add".to_string()));
    }

    #[test]
    fn test_args_inserted_before_user_args() {
        let mut config = create_test_config();
        config.args = vec!["--no-sandbox".to_string(), "--disable-gpu".to_string()];

        let builder = WrappedCommandBuilder::new(config, None);
        let cmd = builder.show("chromium", &["https://example.com".to_string()]);

        // Verify ordering: command, then config args, then user args
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let cmd_idx = parts.iter().position(|p| *p == "chromium").unwrap();
        let no_sandbox_idx = parts.iter().position(|p| *p == "--no-sandbox").unwrap();
        let disable_gpu_idx = parts.iter().position(|p| *p == "--disable-gpu").unwrap();
        let url_idx = parts
            .iter()
            .position(|p| *p == "https://example.com")
            .unwrap();

        assert!(cmd_idx < no_sandbox_idx);
        assert!(no_sandbox_idx < disable_gpu_idx);
        assert!(disable_gpu_idx < url_idx);
    }

    #[test]
    fn test_relative_bind_src_resolved_against_config_dir() {
        let mut config = create_test_config();
        config.bind = vec![("./src".to_string(), "/workspace/src".to_string())];

        let config_dir = Some(PathBuf::from("/home/user/project"));
        let builder = WrappedCommandBuilder::new(config, config_dir);
        let args = builder.build_args();

        let bind_idx = args.iter().position(|x| x == "--bind").unwrap();
        assert_eq!(args[bind_idx + 1], "/home/user/project/src");
        assert_eq!(args[bind_idx + 2], "/workspace/src");
    }

    #[test]
    fn test_absolute_bind_src_unchanged() {
        let mut config = create_test_config();
        config.bind = vec![("/usr".to_string(), "/usr".to_string())];

        let config_dir = Some(PathBuf::from("/home/user/project"));
        let builder = WrappedCommandBuilder::new(config, config_dir);
        let args = builder.build_args();

        let bind_idx = args.iter().position(|x| x == "--bind").unwrap();
        assert_eq!(args[bind_idx + 1], "/usr");
    }

    #[test]
    fn test_relative_bind_without_config_dir_uses_cwd() {
        let mut config = create_test_config();
        config.bind = vec![("./src".to_string(), "/workspace/src".to_string())];

        let builder = WrappedCommandBuilder::new(config, None);
        let args = builder.build_args();

        let bind_idx = args.iter().position(|x| x == "--bind").unwrap();
        assert!(args[bind_idx + 1].ends_with("/src"));
    }

    #[test]
    fn test_relative_chdir_resolved_against_config_dir() {
        let mut config = create_test_config();
        config.chdir = Some(".".to_string());

        let config_dir = Some(PathBuf::from("/home/user/project"));
        let builder = WrappedCommandBuilder::new(config, config_dir);
        let args = builder.build_args();

        let chdir_idx = args.iter().position(|x| x == "--chdir").unwrap();
        assert_eq!(args[chdir_idx + 1], "/home/user/project");
    }

    #[test]
    fn test_sugar_bind_resolves_both_src_and_dst() {
        let mut config = create_test_config();
        config.bind = vec![(".".to_string(), ".".to_string())];

        let config_dir = Some(PathBuf::from("/home/user/project"));
        let builder = WrappedCommandBuilder::new(config, config_dir);
        let args = builder.build_args();

        let bind_idx = args.iter().position(|x| x == "--bind").unwrap();
        assert_eq!(args[bind_idx + 1], "/home/user/project");
        assert_eq!(args[bind_idx + 2], "/home/user/project");
    }
}
