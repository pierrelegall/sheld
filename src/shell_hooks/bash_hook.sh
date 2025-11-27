#!/usr/bin/env bash

# Copyright (C) 2025 Pierre Le Gall
# SPDX-License-Identifier: GPL-3.0-or-later

# Bash hook for Shwrap auto wrapped commands.
# Note: It uses functions as wrappers,
# so user defined functions can be redefined.

typeset -g SHWRAP_PREVIOUS_DIR="$PWD"
typeset -g SHWRAP_COMMANDS=""
typeset -g SHWRAP_DEBUG=${SHWRAP_DEBUG:-0}

# Shwrap logging
__shwrap_log() {
  [[ "$SHWRAP_DEBUG" != "0" ]] && echo "[shwrap] $*" >&2
}

# Wrap command execution
__shwrap_wrap_command() {
  __shwrap_log "Executing command: $@"
  shwrap exec "$@"
}

# Set all commands
__shwrap_set_commands() {
  while IFS= read -r cmd; do
    if [[ -n "$cmd" ]]; then
      __shwrap_log "Set commands: $cmd"
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
  __shwrap_log "Directory change hook called"
  __shwrap_unset_commands
  __shwrap_refresh_commands
  __shwrap_set_commands
}

# Prompt hook
__shwrap_prompt_hook() {
  __shwrap_log "Prompt hook called"
  if [[ "$SHWRAP_PREVIOUS_DIR" != "$PWD" ]]; then
    __shwrap_log "Directory changed detected: $PWD"
    __shwrap_directory_change_hook
    SHWRAP_PREVIOUS_DIR="$PWD"
  fi
}

# Install the hook (preserves existing PROMPT_COMMAND)
if [[ -z "$PROMPT_COMMAND" ]]; then
  PROMPT_COMMAND="__shwrap_prompt_hook"
else
  if [[ "$PROMPT_COMMAND" != *"__shwrap_prompt_hook"* ]]; then
    PROMPT_COMMAND="__shwrap_prompt_hook;$PROMPT_COMMAND"
  fi
fi

# Initial setup
__shwrap_refresh_commands
__shwrap_set_commands
