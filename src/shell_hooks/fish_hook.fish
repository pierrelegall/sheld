#!/usr/bin/env fish

# Copyright (C) 2025 Pierre Le Gall
# SPDX-License-Identifier: GPL-3.0-or-later

# Fish hook for Sheld to auto wrap commands.
# Note: It uses functions as wrappers,
# so user defined functions can be redefined.

set -g SHELD_COMMANDS
set -qg SHELD_DEBUG; or set -g SHELD_DEBUG 0

# Sheld logging
function __sheld_log
  if test "$SHELD_DEBUG" != "0"
    echo "[sheld]" $argv >&2
  end
end

# Wrap command execution
function __sheld_wrap
  __sheld_log "Wrapping:" $argv
  sheld wrap $argv
end

# Update the commands list
function __sheld_update_command_list
  set -g SHELD_COMMANDS (sheld list --simple 2>/dev/null)
end

# Set all commands
function __sheld_set_auto_wraps
  if test -z "$SHELD_COMMANDS"
    return
  end
  for cmd in $SHELD_COMMANDS
    if test -n "$cmd"
      __sheld_log "Set auto-wrap:" $cmd
      eval "
        function $cmd --description 'Sheld sandboxed command'
          __sheld_wrap $cmd \$argv
        end
      "
    end
  end
end

# Unset all commands
function __sheld_unset_auto_wraps
  if test -z "$SHELD_COMMANDS"
    return
  end
  for cmd in $SHELD_COMMANDS
    if test -n "$cmd"
      __sheld_log "Unset auto-wrap:" $cmd
      functions -e $cmd
    end
  end
end

# Refresh all auto wrap commands
function __sheld_refresh_auto_wraps
  __sheld_log "Refresh auto-wraps"
  __sheld_unset_auto_wraps
  __sheld_update_command_list
  __sheld_set_auto_wraps
end

# Directory change hook
function __sheld_directory_change_hook --on-variable PWD
  __sheld_log "Directory changed to:" $PWD
  __sheld_refresh_auto_wraps
end

# Initial setup
__sheld_refresh_auto_wraps
