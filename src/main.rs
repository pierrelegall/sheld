// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

mod cli;
mod shell;

use anyhow::{Context, Result, bail};
use clap::Parser;

use cli::{Action, Cli};
use sheld::bwrap::WrappedCommandBuilder;
use sheld::config::{
    self,
    loader::{ConfigFileError, ConfigLoader},
};
use shell::Shell;

fn main() {
    let input = Cli::parse();

    if let Err(err) = run(input) {
        if !report_invalid_config(&err) {
            eprintln!("Error: {err:?}");
        }
        std::process::exit(1);
    }
}

fn run(input: Cli) -> Result<()> {
    match input.action {
        Action::Init => {
            initialize_config()?;
        }
        Action::Validate { path, silent } => {
            validate_config(path, silent)?;
        }
        Action::List { simple } => {
            list_commands(simple)?;
        }
        Action::Show { command, args } => {
            show_command(&command, &args)?;
        }
        Action::Wrap { command, args } => {
            wrap_command(&command, &args)?;
        }
        Action::Unwrap { command, args } => {
            unwrap_command(&command, &args)?;
        }
        Action::Activate { shell } => {
            print_shell_hook(&shell)?;
        }
        Action::Check { command, silent } => {
            if !check_command(&command, silent)? {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

/// Extract the command name from a path (e.g., "/usr/bin/node" -> "node")
fn get_command_basename(command: &str) -> &str {
    std::path::Path::new(command)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(command)
}

fn wrap_command(command: &str, args: &[String]) -> Result<()> {
    let config = ConfigLoader::load()?.context("No configuration found")?;

    let command_basename = get_command_basename(command);

    let cmd_config = config.get_command(command_basename).context(format!(
        "No configuration found for command '{}'",
        command_basename
    ))?;

    if !cmd_config.enabled {
        bail!(
            "Command '{}' is disabled in configuration",
            command_basename
        );
    }

    let merged_config = config.merge_with_base(cmd_config);
    let effective_command = merged_config
        .alias
        .clone()
        .unwrap_or_else(|| command_basename.to_string());
    let builder = WrappedCommandBuilder::new(merged_config);

    let exit_code = builder.exec(&effective_command, args)?;

    std::process::exit(exit_code)
}

/// Warn the user that a configuration file is invalid and that no command
/// will be sandboxed until it is fixed. Returns `true` if `err` was caused by
/// an invalid configuration file and a warning was printed.
fn report_invalid_config(err: &anyhow::Error) -> bool {
    let Some(file_err) = err.downcast_ref::<ConfigFileError>() else {
        return false;
    };
    let path = file_err.path.display();

    eprintln!("[sheld] WARNING: sheld will NOT sandbox any commands until this is fixed.");
    eprintln!();
    eprintln!("[sheld] Error: failed to load configuration file {}", path);
    eprintln!("[sheld]   {}", err.root_cause());
    eprintln!();
    eprintln!("[sheld] Please fix {}", path);
    if file_err.validate_hint {
        eprintln!(
            "[sheld] (run `sheld validate {}` to check it)",
            shell_quote(&path.to_string())
        );
    }
    true
}

/// Quote `value` for POSIX shells so it can be copy-pasted as one argument.
fn shell_quote(value: &str) -> String {
    let is_safe = |c: char| c.is_ascii_alphanumeric() || "/._-+=:,@%".contains(c);
    if !value.is_empty() && value.chars().all(is_safe) {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn list_commands(simple: bool) -> Result<()> {
    let config = match ConfigLoader::load()? {
        Some(config) => config,
        // In simple mode (used by shell hooks), having no configuration is not
        // an error: there is simply nothing to wrap.
        None if simple => return Ok(()),
        None => bail!("No configuration found"),
    };

    // Sort commands alphabetically
    let commands_map = config.get_commands();
    let mut commands: Vec<_> = commands_map.iter().collect();
    commands.sort_by_key(|(name, _)| *name);

    if simple {
        for (name, cmd_config) in commands {
            if cmd_config.enabled {
                println!("{}", name);
            }
        }
    } else {
        println!("Active command configurations:");
        for (name, cmd_config) in commands {
            if cmd_config.enabled {
                println!("\n{}:", name);
                if !cmd_config.share.is_empty() {
                    let mut shares: Vec<_> = cmd_config.share.iter().collect();
                    shares.sort();
                    println!(
                        "  share: {}",
                        shares.into_iter().cloned().collect::<Vec<_>>().join(", ")
                    );
                }
                if !cmd_config.bind.is_empty() {
                    let mut bind_str: Vec<String> = cmd_config
                        .bind
                        .iter()
                        .map(|(dst, src)| format!("{}:{}", src, dst))
                        .collect();
                    bind_str.sort();
                    println!("  bind: {}", bind_str.join(", "));
                }
            }
        }
    }

    Ok(())
}

fn show_command(command: &str, args: &[String]) -> Result<()> {
    let config = ConfigLoader::load()?.context("No configuration found")?;

    let command_basename = get_command_basename(command);

    let cmd_config = config.get_command(command_basename).context(format!(
        "No configuration found for command '{}'",
        command_basename
    ))?;

    let merged_config = config.merge_with_base(cmd_config);
    let effective_command = merged_config
        .alias
        .clone()
        .unwrap_or_else(|| command_basename.to_string());
    let builder = WrappedCommandBuilder::new(merged_config);

    let cmd_line = builder.show(&effective_command, args);
    println!("{}", cmd_line);

    Ok(())
}

fn unwrap_command(command: &str, args: &[String]) -> Result<()> {
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    // Execute the command directly without any sandboxing
    let mut cmd = Command::new(command);
    cmd.args(args);

    // Use exec to replace the current process
    let error = cmd.exec();

    // If exec returns, it failed
    Err(anyhow::Error::from(error).context(format!("Failed to execute command '{}'", command)))
}

fn validate_config(path: Option<String>, silent: bool) -> Result<()> {
    let config_path = if let Some(p) = path {
        std::path::PathBuf::from(p)
    } else {
        ConfigLoader::get_config_file()?.context("No configuration found")?
    };

    let config = config::Config::from_file(&config_path)?;

    if silent {
        return Ok(());
    }

    println!("Configuration is valid: {:?}", config_path);
    let commands_map = config.get_commands();
    println!("Found {} command(s)", commands_map.len());

    // Sort commands alphabetically
    let mut commands: Vec<_> = commands_map.iter().collect();
    commands.sort_by_key(|(name, _)| *name);

    for (name, cmd_config) in commands {
        match cmd_config.enabled {
            true => println!("  - {}", name),
            false => println!("  - {} (disabled)", name),
        }
    }

    Ok(())
}

fn initialize_config() -> Result<()> {
    use std::fs;

    let template_content = include_str!("../examples/default.yaml");

    let config_path = ConfigLoader::local_config_name();
    // Also fails on a former `.sheld.yaml`, asking to rename it instead.
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    if ConfigLoader::get_local_config_file_in(&current_dir)?.is_some() {
        bail!("{} file already exists in current directory", config_path);
    }

    fs::write(config_path, template_content)
        .context(format!("Failed to write {} file", config_path))?;

    println!("Created {} configuration file", config_path);

    Ok(())
}

fn print_shell_hook(shell_name: &str) -> Result<()> {
    let shell =
        Shell::from_str(shell_name).context(format!("Unsupported shell: {}", shell_name))?;

    let hook = shell
        .get_hook()
        .with_context(|| format!("No hook found for shell {}", shell.to_str()))?;

    print!("{}", hook);

    Ok(())
}

/// Returns whether `command` is configured. Fails if the configuration cannot be loaded.
fn check_command(command: &str, silent: bool) -> Result<bool> {
    let config = ConfigLoader::load()?.context("No configuration found")?;

    let command_basename = get_command_basename(command);

    let command_exists = config.get_command(command_basename).is_some();

    if command_exists {
        if !silent {
            println!("Command `{}` is configured", command_basename);
        }
    } else if !silent {
        eprintln!("Command `{}` not found in configuration", command_basename);
    }

    Ok(command_exists)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_shell_quote() {
        assert_eq!(
            shell_quote("/home/user/.sheld.yaml"),
            "/home/user/.sheld.yaml"
        );
        assert_eq!(shell_quote("/my dir/.sheld.yaml"), "'/my dir/.sheld.yaml'");
        assert_eq!(shell_quote("/it's/.sheld.yaml"), "'/it'\\''s/.sheld.yaml'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn test_get_command_basename_simple_name() {
        assert_eq!(get_command_basename("node"), "node");
        assert_eq!(get_command_basename("npm"), "npm");
        assert_eq!(get_command_basename("python"), "python");
    }

    #[test]
    fn test_get_command_basename_absolute_path() {
        assert_eq!(get_command_basename("/usr/bin/node"), "node");
        assert_eq!(get_command_basename("/usr/local/bin/npm"), "npm");
        assert_eq!(get_command_basename("/bin/python3"), "python3");
    }

    #[test]
    fn test_get_command_basename_with_trailing_slash() {
        assert_eq!(get_command_basename("/usr/bin/"), "bin");
        assert_eq!(get_command_basename("/usr/local/"), "local");
    }

    #[test]
    fn test_get_command_basename_relative_path() {
        assert_eq!(get_command_basename("./node"), "node");
        assert_eq!(get_command_basename("../bin/npm"), "npm");
        assert_eq!(get_command_basename("./some/deep/path/to/cargo"), "cargo");
    }

    #[test]
    fn test_check_command_with_simple_name() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(ConfigLoader::local_config_name());

        let yaml = indoc! {"
            node:
              enabled: true
              share:
                - user
        "};

        fs::write(&config_path, yaml).unwrap();

        let config = config::Config::from_file(&config_path).unwrap();

        assert!(config.get_command("node").is_some());
        assert!(config.get_command("node").unwrap().enabled);

        assert!(config.get_command("python").is_none());
    }

    #[test]
    fn test_check_command_with_absolute_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(ConfigLoader::local_config_name());

        fs::write(
            &config_path,
            indoc! {"
                node:
                  enabled: true
                  share:
                    - user
            "},
        )
        .unwrap();

        let config = config::Config::from_file(&config_path).unwrap();

        assert!(
            config
                .get_command(get_command_basename("/usr/bin/node"))
                .is_some()
        );

        assert!(
            config
                .get_command(get_command_basename("/usr/bin/python"))
                .is_none()
        );
    }

    #[test]
    fn test_check_command_with_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(ConfigLoader::local_config_name());

        let yaml = indoc! {"
            npm:
              enabled: true
              share:
                - network
        "};

        fs::write(&config_path, yaml).unwrap();

        let config = config::Config::from_file(&config_path).unwrap();

        assert!(config.get_command(get_command_basename("./npm")).is_some());
        assert!(
            config
                .get_command(get_command_basename("../bin/npm"))
                .is_some()
        );
    }

    #[test]
    fn test_show_command_with_simple_name() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(ConfigLoader::local_config_name());

        let yaml = indoc! {"
            node:
              enabled: true
              share:
                - user
        "};

        fs::write(&config_path, yaml).unwrap();

        let config = config::Config::from_file(&config_path).unwrap();

        assert!(config.get_command(get_command_basename("node")).is_some());
    }

    #[test]
    fn test_show_command_with_absolute_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(ConfigLoader::local_config_name());

        let yaml = indoc! {"
            node:
              enabled: true
              share:
                - user
        "};

        fs::write(&config_path, yaml).unwrap();

        let config = config::Config::from_file(&config_path).unwrap();

        assert!(
            config
                .get_command(get_command_basename("/usr/bin/node"))
                .is_some()
        );
    }

    #[test]
    fn test_show_command_with_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(ConfigLoader::local_config_name());

        let yaml = indoc! {"
            cargo:
              enabled: true
              share:
                - user
        "};

        fs::write(&config_path, yaml).unwrap();

        let config = config::Config::from_file(&config_path).unwrap();

        assert!(
            config
                .get_command(get_command_basename("./cargo"))
                .is_some()
        );
    }
}
