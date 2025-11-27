#!/usr/bin/env zsh

# Copyright (C) 2025 Pierre Le Gall
# SPDX-License-Identifier: GPL-3.0-or-later

# Zsh hook for Sheld auto wrapped commands.
# Note: It uses functions as wrappers,
# so user defined functions can be redefined.

typeset -g SHELD_COMMANDS=""
typeset -g SHELD_DEBUG=${SHELD_DEBUG:-0}

# Sheld logging
__sheld_log() {
  [[ "$SHELD_DEBUG" != "0" ]] && echo "[sheld] $*" >&2
}

# Wrap command execution
__sheld_wrap_command() {
  __sheld_log "Wrapping: $@"
  sheld wrap "$@"
}

# Set all commands
__sheld_set_commands() {
  while IFS= read -r cmd; do
    if [[ -n "$cmd" ]]; then
      __sheld_log "Set command: $cmd"
      eval "
        $cmd() {
          __sheld_wrap_command $cmd \"\$@\"
        }
      "
    fi
  done <<< "$SHELD_COMMANDS"
}

# Refresh SHELD_COMMANDS variable
__sheld_refresh_commands() {
  SHELD_COMMANDS=$(sheld list --simple 2>/dev/null)
}

# Unset all commands
__sheld_unset_commands() {
  while IFS= read -r cmd; do
    if [[ -n "$cmd" ]]; then
      __sheld_log "Unset command: $cmd"
      unset -f $cmd
    fi
  done <<< "$SHELD_COMMANDS"
}

# Directory change hook
__sheld_directory_change_hook() {
  __sheld_log "Directory changed to: $PWD"
  __sheld_unset_commands
  __sheld_refresh_commands
  __sheld_set_commands
}

# Add our hook to Zsh's chpwd_functions array
if (( ! ${chpwd_functions[(I)__sheld_directory_change_hook]} )); then
  chpwd_functions+=(__sheld_directory_change_hook)
fi

# Initial setup
__sheld_refresh_commands
__sheld_set_commands
