#!/usr/bin/env zsh

# Copyright (C) 2025 Pierre Le Gall
# SPDX-License-Identifier: GPL-3.0-or-later

# Zsh hook for Shwrap auto wrapped commands.
# Note: It uses functions as wrappers,
# so user defined functions can be redefined.

typeset -g SHWRAP_COMMANDS=""
typeset -g SHWRAP_DEBUG=${SHWRAP_DEBUG:-0}

# Shwrap logging
__shwrap_log() {
  [[ "$SHWRAP_DEBUG" != "0" ]] && echo "[shwrap] $*" >&2
}

# Wrap command execution
__shwrap_wrap_command() {
  __shwrap_log "Executing command: $@"
  shwrap wrap "$@"
}

# Set all commands
__shwrap_set_commands() {
  while IFS= read -r cmd; do
    if [[ -n "$cmd" ]]; then
      __shwrap_log "Set command: $cmd"
      eval "
        $cmd() {
          __shwrap_wrap_command $cmd \"\$@\"
        }
      "
    fi
  done <<< "$SHWRAP_COMMANDS"
}

# Refresh SHWRAP_COMMANDS variable
__shwrap_refresh_commands() {
  SHWRAP_COMMANDS=$(shwrap list --simple 2>/dev/null)
}

# Unset all commands
__shwrap_unset_commands() {
  while IFS= read -r cmd; do
    if [[ -n "$cmd" ]]; then
      __shwrap_log "Unset command: $cmd"
      unset -f $cmd
    fi
  done <<< "$SHWRAP_COMMANDS"
}

# Directory change hook
__shwrap_directory_change_hook() {
  __shwrap_log "Directory changed to: $PWD"
  __shwrap_unset_commands
  __shwrap_refresh_commands
  __shwrap_set_commands
}

# Add our hook to Zsh's chpwd_functions array
if (( ! ${chpwd_functions[(I)__shwrap_directory_change_hook]} )); then
  chpwd_functions+=(__shwrap_directory_change_hook)
fi

# Initial setup
__shwrap_refresh_commands
__shwrap_set_commands
