use anyhow::Result;
use std::process::Command;

use crate::config::Entry;

const NAMESPACES: [&str; 6] = ["user", "pid", "network", "ipc", "uts", "cgroup"];

pub struct WrappedCommandBuilder {
    config: Entry,
}

impl WrappedCommandBuilder {
    pub fn new(config: Entry) -> Self {
        Self { config }
    }

    /// Build the bwrap command arguments
    pub fn build_args(&self) -> Vec<String> {
        let mut args = Vec::new();

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
        for bind in &self.config.bind {
            let parts: Vec<&str> = bind.split(':').collect();
            if parts.len() == 2 {
                let src = shellexpand::full(parts[0]).unwrap_or_else(|_| parts[0].into());
                let dst = shellexpand::full(parts[1]).unwrap_or_else(|_| parts[1].into());
                args.push("--bind".to_string());
                args.push(src.to_string());
                args.push(dst.to_string());
            } else {
                eprintln!("Warning: invalid bind format '{}'", bind);
            }
        }

        // Handle read-only binds
        for ro_bind in &self.config.ro_bind {
            let expanded = shellexpand::full(ro_bind).unwrap_or_else(|_| ro_bind.into());
            args.push("--ro-bind".to_string());
            args.push(expanded.to_string());
            args.push(expanded.to_string());
        }

        // Handle device binds
        for dev_bind in &self.config.dev_bind {
            let expanded = shellexpand::full(dev_bind).unwrap_or_else(|_| dev_bind.into());
            args.push("--dev-bind".to_string());
            args.push(expanded.to_string());
            args.push(expanded.to_string());
        }

        // Handle tmpfs
        for tmpfs in &self.config.tmpfs {
            args.push("--tmpfs".to_string());
            args.push(tmpfs.clone());
        }

        // Handle environment variables
        for (key, value) in &self.config.env {
            args.push("--setenv".to_string());
            args.push(key.clone());
            args.push(value.clone());
        }

        // Handle unset environment variables
        for key in &self.config.unset_env {
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
        parts.extend(command_args.iter().cloned());

        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use crate::config::EntryType;

    use super::*;
    use std::collections::HashMap;

    fn create_test_config() -> Entry {
        Entry {
            entry_type: EntryType::Command,
            enabled: true,
            extends: vec![],
            share: vec![],
            bind: vec![],
            ro_bind: vec![],
            dev_bind: vec![],
            tmpfs: vec![],
            env: HashMap::new(),
            unset_env: vec![],
        }
    }

    #[test]
    fn test_build_args_unshare_all_default() {
        let config = create_test_config();
        // Empty config = all namespaces unshared by default

        let builder = WrappedCommandBuilder::new(config);
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

        let builder = WrappedCommandBuilder::new(config);
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
        config.bind = vec!["/src:/dest".to_string()];

        let builder = WrappedCommandBuilder::new(config);
        let args = builder.build_args();

        let bind_idx = args.iter().position(|x| x == "--bind").unwrap();
        assert_eq!(args[bind_idx + 1], "/src");
        assert_eq!(args[bind_idx + 2], "/dest");
    }

    #[test]
    fn test_build_args_ro_bind() {
        let mut config = create_test_config();
        config.ro_bind = vec!["/usr".to_string()];

        let builder = WrappedCommandBuilder::new(config);
        let args = builder.build_args();

        assert!(args.contains(&"--ro-bind".to_string()));
        assert!(args.contains(&"/usr".to_string()));
    }

    #[test]
    fn test_build_args_dev_bind() {
        let mut config = create_test_config();
        config.dev_bind = vec!["/dev/null".to_string()];

        let builder = WrappedCommandBuilder::new(config);
        let args = builder.build_args();

        assert!(args.contains(&"--dev-bind".to_string()));
        assert!(args.contains(&"/dev/null".to_string()));
    }

    #[test]
    fn test_build_args_tmpfs() {
        let mut config = create_test_config();
        config.tmpfs = vec!["/tmp".to_string(), "/var/tmp".to_string()];

        let builder = WrappedCommandBuilder::new(config);
        let args = builder.build_args();

        assert!(args.contains(&"--tmpfs".to_string()));
        assert!(args.contains(&"/tmp".to_string()));
        assert!(args.contains(&"/var/tmp".to_string()));
    }

    #[test]
    fn test_build_args_env() {
        let mut config = create_test_config();
        config
            .env
            .insert("NODE_ENV".to_string(), "production".to_string());
        config.env.insert("DEBUG".to_string(), "true".to_string());

        let builder = WrappedCommandBuilder::new(config);
        let args = builder.build_args();

        let setenv_count = args.iter().filter(|x| *x == "--setenv").count();
        assert_eq!(setenv_count, 2);
        assert!(args.contains(&"NODE_ENV".to_string()));
        assert!(args.contains(&"production".to_string()));
    }

    #[test]
    fn test_build_args_unset_env() {
        let mut config = create_test_config();
        config.unset_env = vec!["DEBUG".to_string(), "VERBOSE".to_string()];

        let builder = WrappedCommandBuilder::new(config);
        let args = builder.build_args();

        assert!(args.contains(&"--unsetenv".to_string()));
        assert!(args.contains(&"DEBUG".to_string()));
        assert!(args.contains(&"VERBOSE".to_string()));
    }

    #[test]
    fn test_build_args_combined() {
        let mut config = create_test_config();
        config.share = vec!["user".to_string()]; // Share only user namespace
        config.ro_bind = vec!["/usr".to_string()];
        config.env.insert("TEST".to_string(), "value".to_string());

        let builder = WrappedCommandBuilder::new(config);
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

        let builder = WrappedCommandBuilder::new(config);
        let cmd = builder.show("node", &["script.js".to_string()]);

        assert!(cmd.starts_with("bwrap"));
        assert!(cmd.contains("--unshare-net"));
        assert!(cmd.contains("node"));
        assert!(cmd.contains("script.js"));
    }

    #[test]
    fn test_show_command_with_multiple_args() {
        let config = create_test_config();
        let builder = WrappedCommandBuilder::new(config);
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
        let builder = WrappedCommandBuilder::new(config);
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
        config.bind = vec!["~/.config:~/.config".to_string()];

        let builder = WrappedCommandBuilder::new(config);
        let args = builder.build_args();

        // shellexpand should expand ~ to home directory
        let bind_idx = args.iter().position(|x| x == "--bind").unwrap();
        // The expanded path should not contain ~
        assert!(!args[bind_idx + 1].contains('~'));
    }

    #[test]
    fn test_invalid_bind_format() {
        let mut config = create_test_config();
        // Invalid bind format (should be src:dest)
        config.bind = vec!["invalid".to_string()];

        let builder = WrappedCommandBuilder::new(config);
        let args = builder.build_args();

        // Should not add invalid bind to args (only warning printed)
        // Count --bind flags, should be 0 for invalid format
        let bind_count = args.iter().filter(|x| *x == "--bind").count();
        assert_eq!(bind_count, 0);
    }

    #[test]
    fn test_unshare_all_by_default() {
        let config = create_test_config();
        let builder = WrappedCommandBuilder::new(config);
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

        let builder = WrappedCommandBuilder::new(config);
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

        let builder = WrappedCommandBuilder::new(config);
        let args = builder.build_args();

        // No namespaces should be unshared
        assert!(!args.contains(&"--unshare-user".to_string()));
        assert!(!args.contains(&"--unshare-pid".to_string()));
        assert!(!args.contains(&"--unshare-net".to_string()));
        assert!(!args.contains(&"--unshare-ipc".to_string()));
        assert!(!args.contains(&"--unshare-uts".to_string()));
        assert!(!args.contains(&"--unshare-cgroup".to_string()));
    }
}
