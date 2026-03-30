#!/usr/bin/env zsh

# Copyright (C) 2025 Pierre Le Gall
# SPDX-License-Identifier: GPL-3.0-or-later

# Zsh hook for Sheld to auto wrap commands.
# Note: It uses functions as wrappers,
# so user defined functions can be redefined.

typeset -g SHELD_COMMANDS=""
typeset -g SHELD_DEBUG=${SHELD_DEBUG:-0}

# Sheld logging
__sheld_log() {
  [[ "$SHELD_DEBUG" != "0" ]] && echo "[sheld] $*" >&2
}

# Wrap command execution
__sheld_wrap() {
  __sheld_log "Wrapping: $@"
  sheld wrap "$@"
}

# Update the commands list
__sheld_update_command_list() {
  SHELD_COMMANDS=$(sheld list --simple 2>/dev/null)
}

# Set all commands
__sheld_set_auto_wraps() {
  if [ -z "$SHELD_COMMANDS" ]; then return; fi
  while IFS= read -r cmd; do
    if [[ -n "$cmd" ]]; then
      __sheld_log "Set auto-wrap: $cmd"
      eval "
        $cmd() {
          __sheld_wrap $cmd \"\$@\"
        }
      "
    fi
  done <<< "$SHELD_COMMANDS"
}

# Unset all commands
__sheld_unset_auto_wraps() {
  if [ -z "$SHELD_COMMANDS" ]; then return; fi
  while IFS= read -r cmd; do
    if [[ -n "$cmd" ]]; then
      __sheld_log "Unset auto-wrap: $cmd"
      unset -f $cmd
    fi
  done <<< "$SHELD_COMMANDS"
}

# Refresh all auto wrap commands
__sheld_refresh_auto_wraps() {
  __sheld_log "Refresh auto-wraps"
  __sheld_unset_auto_wraps
  __sheld_update_command_list
  __sheld_set_auto_wraps
}

# Directory change hook
__sheld_directory_change_hook() {
  __sheld_log "Directory changed to: $PWD"
  __sheld_refresh_auto_wraps
}

# Add our hook to Zsh's chpwd_functions array
if [[ -z "${chpwd_functions[(r)__sheld_directory_change_hook]}" ]]; then
  chpwd_functions+=(__sheld_directory_change_hook)
fi

# Initial setup
__sheld_refresh_auto_wraps
