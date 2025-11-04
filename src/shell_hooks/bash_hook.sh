#!/usr/bin/env bash

# Copyright (C) 2025 Pierre Le Gall
# SPDX-License-Identifier: GPL-3.0-or-later

# Bash hook for Shwrap auto wrapped commands.
# Note: It use functions as aliases,
# so user defined functions can be reset/unset if exist.

typeset -g SHWRAP_PREV_PWD="$PWD"
typeset -g SHWRAP_WRAPPED_COMMANDS=""
typeset -g SHWRAP_DEBUG=${SHWRAP_DEBUG:-0}

# Shwrap logging function
__shwrap_log() {
  [[ "$SHWRAP_DEBUG" != "0" ]] && echo "[shwrap] $*" >&2
}

# Wrap command execution
__shwrap_wrap_command() {
  __shwrap_log "Executing wrapped command: $@"
  shwrap command exec "$@"
}

# Set all wrapped commands
__shwrap_set_wrapped_commands() {
  while IFS= read -r cmd; do
    if [[ -n "$cmd" ]]; then
      __shwrap_log "Set wrapped commands: $cmd"
      eval "
        $cmd() {
          __shwrap_wrap_command $cmd \"\$@\"
        }
      "
    fi
  done <<< "$SHWRAP_WRAPPED_COMMANDS" 
}

# Refresh SHWRAP_WRAPPED_COMMANDS variable
__shwrap_refresh_wrapped_commands() {
  SHWRAP_WRAPPED_COMMANDS=$(shwrap command list --simple 2>/dev/null)
}

# Unset all wrapped commands
__shwrap_unset_wrapped_commands() {
  while IFS= read -r cmd; do
    if [[ -n "$cmd" ]]; then
      __shwrap_log "Unset wrapped command: $cmd"
      unset -f $cmd
    fi
  done <<< "$SHWRAP_WRAPPED_COMMANDS"
}

# Hook function that runs on directory change
__shwrap_on_directory_change() {
  __shwrap_unset_wrapped_commands
  __shwrap_refresh_wrapped_commands
  __shwrap_set_wrapped_commands
}

# Main hook that detects PWD changes
__shwrap_chpwd_hook() {
  __shwrap_log "CHPWD hook called"
  if [[ "$SHWRAP_PREV_PWD" != "$PWD" ]]; then
    __shwrap_log "Directory changed detected: $PWD"
    __shwrap_on_directory_change
    SHWRAP_PREV_PWD="$PWD"
  fi
}

# Install the hook (preserves existing PROMPT_COMMAND)
if [[ -z "$PROMPT_COMMAND" ]]; then
  PROMPT_COMMAND="__shwrap_chpwd_hook"
else
  # Only add if not already present
  if [[ "$PROMPT_COMMAND" != *"__shwrap_chpwd_hook"* ]]; then
    PROMPT_COMMAND="__shwrap_chpwd_hook;$PROMPT_COMMAND"
  fi
fi

__shwrap_refresh_wrapped_commands
__shwrap_set_wrapped_commands
