mod cli;
mod hooks;

use anyhow::{bail, Context, Result};
use clap::Parser;

use bwrap_manager::bwrap::BwrapBuilder;
use bwrap_manager::config::{self, loader::ConfigLoader};
use cli::{Cli, Commands};
// use hooks::Shell;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { template } => {
            cmd_init(template)?;
        }
        Commands::Exec {
            command,
            args,
            dry_run,
        } => {
            cmd_exec(&command, &args, dry_run)?;
        }
        Commands::Check { path } => {
            cmd_check(path)?;
        }
        Commands::List => {
            cmd_list()?;
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

    let config_path = ".bwrap";
    if std::path::Path::new(config_path).exists() {
        bail!(".bwrap file already exists in current directory");
    }

    fs::write(config_path, template_content).context("Failed to write .bwrap file")?;

    println!("Created .bwrap configuration file");

    Ok(())
}

fn cmd_exec(command: &str, args: &[String], dry_run: bool) -> Result<()> {
    let config = ConfigLoader::load()?.context("No .bwrap configuration found")?;

    let cmd_config = config
        .get_command_config(command)
        .context(format!("No configuration found for command '{}'", command))?;

    if !cmd_config.enabled {
        bail!("Command '{}' is disabled in configuration", command);
    }

    let merged_config = config.merge_with_base(cmd_config);
    let builder = BwrapBuilder::new(merged_config);

    if dry_run {
        let cmd_line = builder.show(command, args);
        println!("{}", cmd_line);
        Ok(())
    } else {
        let exit_code = builder.exec(command, args)?;
        std::process::exit(exit_code);
    }
}

fn cmd_check(path: Option<String>) -> Result<()> {
    let config_path = if let Some(p) = path {
        std::path::PathBuf::from(p)
    } else {
        ConfigLoader::find_config()?.context("No .bwrap configuration found")?
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

fn cmd_list() -> Result<()> {
    let config = ConfigLoader::load()?.context("No .bwrap configuration found")?;

    // Sort commands alphabetically
    let mut commands: Vec<_> = config.commands.iter().collect();
    commands.sort_by_key(|(name, _)| *name);

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

    Ok(())
}

fn cmd_which() -> Result<()> {
    if let Some(config_path) = ConfigLoader::find_config()? {
        println!("{}", config_path.display());
    } else {
        println!("No .bwrap configuration found");
    }

    Ok(())
}

fn cmd_show(command: &str, args: &[String]) -> Result<()> {
    let config = ConfigLoader::load()?.context("No .bwrap configuration found")?;

    let cmd_config = config
        .get_command_config(command)
        .context(format!("No configuration found for command '{}'", command))?;

    let merged_config = config.merge_with_base(cmd_config);
    let builder = BwrapBuilder::new(merged_config);

    let cmd_line = builder.show(command, args);
    println!("{}", cmd_line);

    Ok(())
}

fn cmd_shell_hook(_shell_name: &str) -> Result<()> {
    // let shell =
    //     Shell::from_str(shell_name).context(format!("Unsupported shell: {}", shell_name))?;

    // let hook = shell.get_hook();
    // let script = hook.generate()?;

    // print!("{}", script);

    print!("Command not yet supported");

    Ok(())
}
