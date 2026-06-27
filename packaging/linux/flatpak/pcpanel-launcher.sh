#!/bin/sh
# Flatpak entry point. Launches the Tauri desktop shell, which opens the native window and spawns the
# backend native image as a child process.
#
# PCPANEL_BACKEND_DIR tells the shell exactly where the backend lives (the native image loads its
# companion shared libraries from its own directory), so we don't depend on Tauri's resource-dir
# heuristics inside this hand-rolled layout.
#
# PCPANEL_SHELL marks the backend as shell-hosted (suppresses its own browser-open); the shell also sets
# this on the child, but exporting it here is harmless and explicit.
#
# Point the kdotool command at the wrapper (which sets a host-visible TMPDIR so KWin can read the
# script kdotool generates). The backend inherits this env from the shell. An explicit path
# (contains '/') is honoured verbatim, so this beats the bundled-sibling lookup. LINUX_COMMANDS_KDOTOOL
# maps to the `linux.commands.kdotool` config property.
exec env \
  PCPANEL_BACKEND_DIR=/app/pcpanel/backend \
  PCPANEL_SHELL=flatpak \
  LINUX_COMMANDS_KDOTOOL=/app/bin/kdotool \
  /app/pcpanel/pcpanel-shell "$@"
