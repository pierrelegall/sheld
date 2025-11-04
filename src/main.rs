// Copyright (C) 2025 Pierre Le Gall
// SPDX-License-Identifier: GPL-3.0-or-later

mod cli;
mod shell_hooks;

use anyhow::{bail, Context, Result};
use clap::Parser;

use cli::{Cli, Commands};
use shell_hooks::Shell;
use shwrap::bwrap::BwrapBuilder;
use shwrap::config::{self, loader::ConfigLoader};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { template } => {
            cmd_init(template)?;
        }
        Commands::Exec { command, args } => {
            cmd_exec(&command, &args)?;
        }
        Commands::Check { path } => {
            cmd_check(path)?;
        }
        Commands::List { simple } => {
            cmd_list(simple)?;
        }
        Commands::Which => {
            cmd_which()?;
        }
        Commands::Show { command, args } => {
            cmd_show(&command, &args)?;
        }
        Commands::ShellHook { shell } => {
            cmd_shell_hook(&shell)?;
        }
    }

    Ok(())
}

fn cmd_init(template: Option<String>) -> Result<()> {
    use std::fs;

    let template_content = match template.as_deref() {
        Some("nodejs") => include_str!("../templates/nodejs.yaml"),
        Some("python") => include_str!("../templates/python.yaml"),
        Some("ruby") => include_str!("../templates/ruby.yaml"),
        Some("go") => include_str!("../templates/go.yaml"),
        Some("rust") => include_str!("../templates/rust.yaml"),
        None => include_str!("../templates/default.yaml"),
        Some(other) => bail!("Unknown template: {}", other),
    };

    let config_path = ".shwrap";
    if std::path::Path::new(config_path).exists() {
        bail!(".shwrap file already exists in current directory");
    }

    fs::write(config_path, template_content).context("Failed to write .shwrap file")?;

    println!("Created .shwrap configuration file");

    Ok(())
}

fn cmd_exec(command: &str, args: &[String]) -> Result<()> {
    let config = ConfigLoader::load()?.context("No .shwrap configuration found")?;

    let cmd_config = config
        .get_command_config(command)
        .context(format!("No configuration found for command '{}'", command))?;

    if !cmd_config.enabled {
        bail!("Command '{}' is disabled in configuration", command);
    }

    let merged_config = config.merge_with_base(cmd_config);
    let builder = BwrapBuilder::new(merged_config);

    let exit_code = builder.exec(command, args)?;

    std::process::exit(exit_code)
}

fn cmd_check(path: Option<String>) -> Result<()> {
    let config_path = if let Some(p) = path {
        std::path::PathBuf::from(p)
    } else {
        ConfigLoader::find_config()?.context("No .shwrap configuration found")?
    };

    let config = config::BwrapConfig::from_file(&config_path)?;
    println!("Configuration is valid: {:?}", config_path);
    println!("Found {} command(s)", config.commands.len());

    // Sort commands alphabetically
    let mut commands: Vec<_> = config.commands.iter().collect();
    commands.sort_by_key(|(name, _)| *name);

    for (name, cmd_config) in commands {
        match cmd_config.enabled {
            true => println!("  - {}", name),
            false => println!("  - {} (disabled)", name),
        }
    }

    Ok(())
}

fn cmd_list(simple: bool) -> Result<()> {
    let config = ConfigLoader::load()?.context("No .shwrap configuration found")?;

    // Sort commands alphabetically
    let mut commands: Vec<_> = config.commands.iter().collect();
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
                    println!("  bind: {}", cmd_config.bind.join(", "));
                }
            }
        }
    }

    Ok(())
}

fn cmd_which() -> Result<()> {
    if let Some(config_path) = ConfigLoader::find_config()? {
        println!("{}", config_path.display());
    } else {
        println!("No .shwrap configuration found");
    }

    Ok(())
}

fn cmd_show(command: &str, args: &[String]) -> Result<()> {
    let config = ConfigLoader::load()?.context("No .shwrap configuration found")?;

    let cmd_config = config
        .get_command_config(command)
        .context(format!("No configuration found for command '{}'", command))?;

    let merged_config = config.merge_with_base(cmd_config);
    let builder = BwrapBuilder::new(merged_config);

    let cmd_line = builder.show(command, args);
    println!("{}", cmd_line);

    Ok(())
}

fn cmd_shell_hook(shell_name: &str) -> Result<()> {
    let shell =
        Shell::from_str(shell_name).context(format!("Unsupported shell: {}", shell_name))?;

    let hook = shell
        .get_hook()
        .with_context(|| format!("No hook found for shell {}", shell.to_str()))?;

    print!("{}", hook);

    Ok(())
}
