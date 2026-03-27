// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

mod cli;
mod shell_hooks;

use anyhow::{Context, Result, bail};
use clap::Parser;

use cli::{Action, Cli};
use sheld::bwrap::WrappedCommandBuilder;
use sheld::config::{self, loader::ConfigLoader};
use shell_hooks::Shell;

fn main() -> Result<()> {
    let input = Cli::parse();

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
        Action::Bypass { command, args } => {
            bypass_command(&command, &args)?;
        }
        Action::Activate { shell } => {
            print_shell_hook(&shell)?;
        }
        Action::Check { command, silent } => {
            if check_command(&command, silent).is_err() {
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
    let builder = WrappedCommandBuilder::new(merged_config);

    let exit_code = builder.exec(command, args)?;

    std::process::exit(exit_code)
}

fn list_commands(simple: bool) -> Result<()> {
    let config = ConfigLoader::load()?.context("No configuration found")?;

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
                    println!("  share: {}", cmd_config.share.join(", "));
                }
                if !cmd_config.bind.is_empty() {
                    let bind_str: Vec<String> = cmd_config
                        .bind
                        .iter()
                        .map(|(src, dst)| format!("{}:{}", src, dst))
                        .collect();
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
    let builder = WrappedCommandBuilder::new(merged_config);

    let cmd_line = builder.show(command, args);
    println!("{}", cmd_line);

    Ok(())
}

fn bypass_command(command: &str, args: &[String]) -> Result<()> {
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
    if std::path::Path::new(config_path).exists() {
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

fn check_command(command: &str, silent: bool) -> Result<()> {
    let config = ConfigLoader::load()?.context("No configuration found")?;

    let command_basename = get_command_basename(command);

    let command_exists = config.get_command(command_basename).is_some();

    if command_exists {
        if !silent {
            println!("Command `{}` is configured", command_basename);
        }
        Ok(())
    } else {
        if !silent {
            eprintln!("Command `{}` not found in configuration", command_basename);
        }
        bail!("Command not found")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use std::fs;
    use tempfile::TempDir;

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
