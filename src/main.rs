// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

mod cli;
mod shell_hooks;

use anyhow::{Context, Result, bail};
use clap::Parser;

use cli::{Cli, CommandAction, ConfigAction, ShellHookAction, Subject};
use shell_hooks::Shell;
use shwrap::bwrap::WrappedCommandBuilder;
use shwrap::config::{self, loader::ConfigLoader};

fn main() -> Result<()> {
    let input = Cli::parse();

    match input.subject {
        Subject::Config { action } => match action {
            ConfigAction::Init => {
                config_init_cmd()?;
            }
            ConfigAction::Check { path, silent } => {
                config_check_cmd(path, silent)?;
            }
            ConfigAction::Which => {
                config_which_cmd()?;
            }
        },
        Subject::Command { action } => match action {
            CommandAction::List { simple } => {
                command_list_cmd(simple)?;
            }
            CommandAction::Exec { command, args } => {
                command_exec_cmd(&command, &args)?;
            }
            CommandAction::Show { command, args } => {
                command_show_cmd(&command, &args)?;
            }
            CommandAction::Bypass { command, args } => {
                command_bypass_cmd(&command, &args)?;
            }
        },
        Subject::ShellHook { action } => match action {
            ShellHookAction::Get { shell } => {
                shell_hook_get_cmd(&shell)?;
            }
        },
    }

    Ok(())
}

fn command_exec_cmd(command: &str, args: &[String]) -> Result<()> {
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

fn command_list_cmd(simple: bool) -> Result<()> {
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
                    let bind_str: Vec<String> = cmd_config.bind.iter()
                        .map(|(src, dst)| format!("{}:{}", src, dst))
                        .collect();
                    println!("  bind: {}", bind_str.join(", "));
                }
            }
        }
    }

    Ok(())
}

fn command_show_cmd(command: &str, args: &[String]) -> Result<()> {
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

fn command_bypass_cmd(command: &str, args: &[String]) -> Result<()> {
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

fn config_check_cmd(path: Option<String>, silent: bool) -> Result<()> {
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

fn config_init_cmd() -> Result<()> {
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

fn config_which_cmd() -> Result<()> {
    if let Some(config_path) = ConfigLoader::get_config_file()? {
        println!("{}", config_path.display());
    } else {
        println!("No configuration found");
    }

    Ok(())
}

fn shell_hook_get_cmd(shell_name: &str) -> Result<()> {
    let shell =
        Shell::from_str(shell_name).context(format!("Unsupported shell: {}", shell_name))?;

    let hook = shell
        .get_hook()
        .with_context(|| format!("No hook found for shell {}", shell.to_str()))?;

    print!("{}", hook);

    Ok(())
}
