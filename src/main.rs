// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

mod cli;
mod shell_hooks;

use anyhow::{Context, Result, bail};
use clap::Parser;

use cli::{Action, Cli};
use shell_hooks::Shell;
use shwrap::bwrap::WrappedCommandBuilder;
use shwrap::config::{self, loader::ConfigLoader};

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
            check_command(&command, silent)?;
        }
    }

    Ok(())
}

fn wrap_command(command: &str, args: &[String]) -> Result<()> {
    let config = ConfigLoader::load()?.context("No configuration found")?;

    let cmd_config = config
        .get_command(command)
        .context(format!("No configuration found for command '{}'", command))?;

    if !cmd_config.enabled {
        bail!("Command '{}' is disabled in configuration", command);
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

    let cmd_config = config
        .get_command(command)
        .context(format!("No configuration found for command '{}'", command))?;

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

    let exists = config.get_command(command).is_some();

    if exists {
        if !silent {
            println!("Command `{}` is configured", command);
        }
        Ok(())
    } else {
        if !silent {
            eprintln!("Command `{}` not found in configuration", command);
        }
        std::process::exit(1)
    }
}
