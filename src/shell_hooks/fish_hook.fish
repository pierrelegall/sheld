#!/usr/bin/env fish

# Copyright (C) 2025 Pierre Le Gall
# SPDX-License-Identifier: GPL-3.0-or-later

# Fish hook for Sheld auto wrapped commands.
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
function __sheld_wrap_command
  __sheld_log "Wrapping:" $argv
  sheld wrap $argv
end

# Set all commands
function __sheld_set_commands
  for cmd in $SHELD_COMMANDS
    if test -n "$cmd"
      __sheld_log "Set command:" $cmd
      eval "
        function $cmd --description 'Sheld sandboxed command'
          __sheld_wrap_command $cmd \$argv
        end
      "
    end
  end
end

# Refresh SHELD_COMMANDS variable
function __sheld_refresh_commands
  set -g SHELD_COMMANDS (sheld list --simple 2>/dev/null)
end

# Unset all commands
function __sheld_unset_commands
  for cmd in $SHELD_COMMANDS
    if test -n "$cmd"
      __sheld_log "Unset command:" $cmd
      functions -e $cmd
    end
  end
end

# Directory change hook
function __sheld_directory_change_hook --on-variable PWD
  __sheld_log "Directory changed to:" $PWD
  __sheld_unset_commands
  __sheld_refresh_commands
  __sheld_set_commands
end

# Initial setup
__sheld_refresh_commands
__sheld_set_commands
