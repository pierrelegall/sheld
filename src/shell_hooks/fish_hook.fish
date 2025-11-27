#!/usr/bin/env fish

# Copyright (C) 2025 Pierre Le Gall
# SPDX-License-Identifier: GPL-3.0-or-later

# Fish hook for Shwrap auto wrapped commands.
# Note: It uses functions as wrappers,
# so user defined functions can be redefined.

set -g SHWRAP_COMMANDS
set -qg SHWRAP_DEBUG; or set -g SHWRAP_DEBUG 0

# Shwrap logging
function __shwrap_log
  if test "$SHWRAP_DEBUG" != "0"
    echo "[shwrap]" $argv >&2
  end
end

# Wrap command execution
function __shwrap_wrap_command
  __shwrap_log "Executing command:" $argv
  shwrap wrap $argv
end

# Set all commands
function __shwrap_set_commands
  for cmd in $SHWRAP_COMMANDS
    if test -n "$cmd"
      __shwrap_log "Set command:" $cmd
      eval "
        function $cmd --description 'Shwrap sandboxed command'
          __shwrap_wrap_command $cmd \$argv
        end
      "
    end
  end
end

# Refresh SHWRAP_COMMANDS variable
function __shwrap_refresh_commands
  set -g SHWRAP_COMMANDS (shwrap list --simple 2>/dev/null)
end

# Unset all commands
function __shwrap_unset_commands
  for cmd in $SHWRAP_COMMANDS
    if test -n "$cmd"
      __shwrap_log "Unset command:" $cmd
      functions -e $cmd
    end
  end
end

# Directory change hook
function __shwrap_directory_change_hook --on-variable PWD
  __shwrap_log "Directory changed to:" $PWD
  __shwrap_unset_commands
  __shwrap_refresh_commands
  __shwrap_set_commands
end

# Initial setup
__shwrap_refresh_commands
__shwrap_set_commands
