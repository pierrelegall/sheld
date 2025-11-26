<h3 align="center">
  shwrap
</h3>

<div align="center">
  <a href="https://github.com/pierrelegall/shwrap/stargazers"><img src="https://img.shields.io/github/stars/pierrelegall/shwrap?colorA=363a4f&colorB=b7bdf8&style=for-the-badge"></a>
  <a href="https://github.com/pierrelegall/shwrap/issues"><img src="https://img.shields.io/github/issues/pierrelegall/shwrap?colorA=363a4f&colorB=f5a97f&style=for-the-badge"></a>
  <a href="https://github.com/pierrelegall/shwrap/contributors"><img src="https://img.shields.io/github/contributors/pierrelegall/shwrap?colorA=363a4f&colorB=a6da95&style=for-the-badge"></a>
</div>

## About

Shwrap allows you to define sandbox profiles (in your directory or globally for your user) for different commands and automatically wraps them using [Bubblewrap](https://github.com/containers/bubblewrap) when executed. Hooks are available for `bash`, `zsh`, and `fish`.

⚠ **Alpha software**: Shwrap is an alpha software, so breaking changes will happen.

## Features

- 📁 **Hierarchical configuration**: Local `.shwrap.yaml` merges with user config at `~/.config/shwrap/default.yaml`
- 🔒 **Secure by default**: All namespaces unshared unless explicitly allowed
- 🎯 **Per-command rules**: Different sandbox settings for each command
- 📦 **Model system**: Reusable configuration models for common patterns
- 🔄 **Shell integration**: Automatic command wrapping via shell hooks

## Installation

Build from source:

```sh
git clone https://github.com/pierrelegall/shwrap.git
cd shwrap
cargo build --release
```

## How to setup command wrapping

First, initialize a configuration file in a directory:

```sh
shwrap config init
```

Then, edit the `.shwrap.yaml` file to define your command wraps:

```yaml
node:
  share:
    - user
    - network
  bind:
    - ~/.npm:~/.npm
    - $PWD:/workspace
  ro_bind:
    - /usr
    - /lib
```

## How to run wrapped commands

You can run wrapped commands manually:

```sh
shwrap command exec node app.js
```

Or use the shell hook. Shell hook automatically wrap configured commands when you execute them. It automatically reloads command configurations on directory change.

To bypass the hook system and run a command without sandboxing:

```sh
shwrap command bypass node app.js
```

**Note**: To enable debug logs, set `SHWRAP_DEBUG` to `1`.

## Setup shell hook

### Bash

Add to your `~/.bashrc`:

```sh
eval "$(shwrap shell-hook get bash)"
```

### Zsh

Add to your `~/.zshrc`:

```sh
eval "$(shwrap shell-hook get zsh)"
```

### Fish

Add to your `~/.config/fish/config.fish`:

```sh
shwrap shell-hook get fish | source
```

## Configuration

### Configuration file hierarchy

Shwrap merges configuration files to combine global defaults with project-specific settings:

1. **User**: `~/.config/shwrap/default.yaml` - Global baseline configuration
2. **Local**: `.shwrap.yaml` in current directory or parent directories - Project-specific overrides

When both files exist, they are merged with local entries taking precedence:
- Commands/models with the same name:
  - `override: false` (default): Deep merge (parent + child settings combined)
  - `override: true`: Child completely replaces parent
- Distinct commands/models: both are included
- Local `enabled: false`: use user version instead (skip local override)
- Local commands can extend models defined in user config
- Deep merge behavior:
  - Arrays: Parent items first, then unique child items (deduplicated)
  - env HashMap: Parent + child, child wins on key conflicts
  - Scalar fields: Child value wins

### Configuration syntax

```yaml
# Define reusable models
base:
  type: model               # Mark this as a model (not a command)
  share:
    - user
  ro_bind:
    - /usr
    - /lib

network:
  type: model
  share:
    - network

# Define command-specific configurations
node:
  extends: base             # Optional: extend a single model
  # OR
  extends: [base, network]  # Optional: extend multiple models (applied in order)
  enabled: true             # Optional: enable this command (default: true)
  override: false           # Optional: false=deep merge with parent, true=replace parent (default: false)
  bind:                     # Read-write mounts
    - ~/.npm:~/.npm
    - $PWD:/workspace
  ro_bind:                  # Read-only mounts
    - /etc/resolv.conf
  dev_bind:                 # Device bind mounts
    - /dev/null
  bind_try:                 # Optional: bind mounts that won't fail if source doesn't exist
    - ~/.cache:~/.cache
  ro_bind_try:              # Optional: read-only bind-try mounts
    - /usr/share/fonts
  dev_bind_try:             # Optional: device bind-try mounts
    - /dev/kvm
  tmpfs:                    # Temporary filesystems
    - /tmp
  chdir: /workspace         # Optional: working directory inside sandbox
  die_with_parent: true     # Optional: kill process when parent dies (default: false)
  new_session: false        # Optional: create new terminal session (default: false)
  cap:                      # Optional: add Linux capabilities (bwrap drops all by default)
    - CAP_SYS_ADMIN
    - CAP_NET_ADMIN
  env:                      # Set environment variables
    NODE_ENV: production
  unset_env:                # Unset environment variables
    - DEBUG
```

### Namespace Isolation

By default, **all namespaces are unshared** (isolated). Use `share` to selectively allow:

- `user` - User/group IDs
- `network` - Network access
- `pid` - Process IDs
- `ipc` - Inter-process communication
- `uts` - Hostname
- `cgroup` - Control groups

### Templates

Available templates (use with `shwrap config init --template <name>`):

- `default` - Minimal starter template
- `nodejs` - Node.js development
- `python` - Python development
- `ruby` - Ruby development
- `go` - Go development
- `rust` - Rust development

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## See Also

- [Bubblewrap](https://github.com/containers/bubblewrap) - The underlying sandboxing tool
- [Bubblejail](https://github.com/igo95862/bubblejail) - Alternative sandboxing solution
- [Firejail](https://github.com/netblue30/firejail) - Alternative sandboxing solution

<p align="center">
  Copyright &copy; 2025 <a href="https://github.com/pierrelegall" target="_blank">Pierre Le Gall</a>
</p>

<p align="center">
  <a href="https://github.com/pierrelegall/shwrap/blob/main/LICENSE.md"><img src="https://img.shields.io/static/v1.svg?style=for-the-badge&label=License&message=GPL%20v3&logoColor=d9e0ee&colorA=363a4f&colorB=b7bdf8"/></a>
</p>
