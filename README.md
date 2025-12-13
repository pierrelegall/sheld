<h3 align="center">
  SHELD
</h3>

<div align="center">
  <a href="https://github.com/pierrelegall/sheld/stargazers"><img src="https://img.shields.io/github/stars/pierrelegall/sheld?colorA=363a4f&colorB=b7bdf8&style=for-the-badge"></a>
  <a href="https://github.com/pierrelegall/sheld/issues"><img src="https://img.shields.io/github/issues/pierrelegall/sheld?colorA=363a4f&colorB=f5a97f&style=for-the-badge"></a>
  <a href="https://github.com/pierrelegall/sheld/contributors"><img src="https://img.shields.io/github/contributors/pierrelegall/sheld?colorA=363a4f&colorB=a6da95&style=for-the-badge"></a>
</div>

## About

Sheld (from "shell" and "shield") allows you to define sandbox profiles (user global or directory local) for different commands and automatically wraps them using [Bubblewrap](https://github.com/containers/bubblewrap) when executed. Full integration is available for `bash`, `zsh`, and `fish`.

⚠ **Alpha software**: Sheld is alpha software, so breaking changes will happen.

## Why this tool?

Sheld is designed for these sandboxing use cases:
- **Development environment isolation**: Your dev environment contains hundreds or thousands of dependencies from different package managers and different projects. Each dependency can execute arbitrary code during installation or runtime.
- **AI-powered development tools**: AI coding assistants execute shell commands autonomously. They can make mistakes, be manipulated by prompt injection, or accidentally expose secrets.
- **Tools with MCP server**: Model Context Protocol (MCP) servers are executables that AI assistants call to access filesystems, databases, APIs, and other resources. Third-party MCP servers run with your user permissions and can be exploited through prompt injection.

## Features

- 🎯 **File-based config**: Per-command sandboxing rules in YAML
- 📁 **Hierarchical config**: Global defaults + per-project overrides
- 🔒 **Secure by default**: Commands run fully isolated unless explicitly allowed
- 🔄 **Shell integration**: Configured commands work like normal commands

## Installation

Build from source:

```sh
git clone https://github.com/pierrelegall/sheld.git
cd sheld
cargo build --release
```

## Quick Start

Initialize a configuration file:

```sh
sheld init
```

Edit the `.sheld.yaml` file to define your command wraps:

```yaml
node:
  share:
    - network
  bind:
    - ~/.node_modules
    - [$PWD, /workspace]
  ro_bind:
    - /usr
    - /lib
```

Run commands manually with:

```sh
sheld wrap node script.js
```

## Auto wrapping

Shell hooks allow automatic command wrapping. To set it up, do:

```sh
# For Bash
eval "$(sheld activate bash)"

# For Zsh
eval "$(sheld activate zsh)"

# For Fish
sheld activate fish | source
```

Then, all configured commands will be hooked into a wrapped command, like below:

```sh
# Will run `sheld wrap node script.js`
node script.js
```

Configured commands can now be bypassed with:

```sh
# Wrapping bypassed
sheld bypass node script.js
```

## CLI help

For more information about the CLI, run `sheld help`.

## Config files

### Example

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
  includes: base            # Optional: include a single model
  # OR
  includes: [base, network] # Optional: include multiple models (applied in order)
  enabled: true             # Optional: enable this command (default: true)
  override: false           # Optional: false=deep merge with parent, true=replace parent (default: false)
  bind:                     # Read-write mounts
    - ~/.node_modules
    - [$PWD, /workspace]
  ro_bind:                  # Read-only mounts
    - /etc/resolv.conf
  dev_bind:                 # Device bind mounts
    - /dev/null
  bind_try:                 # Optional: bind mounts that won't fail if source doesn't exist
    - ~/.cache
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

### File hierarchy

Sheld merges configuration files to combine user global defaults with project-specific settings:

1. **User**: `~/.config/sheld/default.yaml` - Global baseline configuration
2. **Local**: `.sheld.yaml` in current directory or parent directories - Project-specific overrides

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
  - Scalar fields: child value wins

## Default isolation

By default, **all namespaces are unshared** following the principle of least privilege. Use `share` to selectively allow namespaces:
- `network` - Network access
- `user` - User/group IDs
- `pid` - Process IDs
- `ipc` - Inter-process communication
- `uts` - Hostname
- `cgroup` - Control groups

If you want a command to behave exactly like bwrap's default (sharing all namespaces), configure it like this:

```yaml
fucmd:
  share:
    - user    # Same user/group IDs as the host
    - network # Full network access (same as host)
    - pid     # Visibility to all host processes
    - ipc     # Access to host IPC mechanisms
    - uts     # Same hostname as the host
    - cgroup  # Access to host cgroups
```

Or use the model system:

```yaml
share_all_namespaces:
  type: model
  share:
    - user    # Same user/group IDs as the host
    - network # Full network access (same as host)
    - pid     # Visibility to all host processes
    - ipc     # Access to host IPC mechanisms
    - uts     # Same hostname as the host
    - cgroup  # Access to host cgroups

fucmd:
  includes: share_all_namespaces

barcmd:
  includes: share_all_namespaces
```

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

<p align="center">
  Copyright &copy; 2025 <a href="https://github.com/pierrelegall" target="_blank">Pierre Le Gall</a>
</p>

<p align="center">
  <a href="https://github.com/pierrelegall/sheld/blob/main/LICENSE.md"><img src="https://img.shields.io/static/v1.svg?style=for-the-badge&label=License&message=GPL%20v3&logoColor=d9e0ee&colorA=363a4f&colorB=b7bdf8"/></a>
</p>
