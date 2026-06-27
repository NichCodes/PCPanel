#!/usr/bin/env bash
# Assemble the Quarkus native-image distribution into src-tauri/backend/ so the Tauri shell can bundle
# it as a resource and spawn it. Run from the repo root AFTER `./mvnw package -Pnative`.
#
# Copies the *-runner(.exe) renamed to PCPanel(.exe) plus every companion native lib next to it
# (the native image is not self-contained — it loads *.so/*.dll/*.jnilib from its own directory).
#
# Usage: packaging/assemble-backend.sh [target-dir] [dest-dir]
set -euo pipefail

target_dir="${1:-target}"
dest_dir="${2:-src-tauri/backend}"

mkdir -p "$dest_dir"
# Clear previous contents but keep the tracked placeholder/readme.
find "$dest_dir" -mindepth 1 ! -name '.gitkeep' ! -name 'README.md' -exec rm -rf {} +

# Locate the native runner (Linux/macOS: *-runner ; Windows: *-runner.exe).
runner=""
for candidate in "$target_dir"/*-runner.exe "$target_dir"/*-runner; do
  if [[ -f "$candidate" ]]; then runner="$candidate"; break; fi
done
if [[ -z "$runner" ]]; then
  echo "error: no *-runner(.exe) found in $target_dir — run ./mvnw package -Pnative first" >&2
  exit 1
fi

case "$runner" in
  *.exe) cp "$runner" "$dest_dir/PCPanel.exe" ;;
  *)     cp "$runner" "$dest_dir/PCPanel"; chmod +x "$dest_dir/PCPanel" ;;
esac
echo "copied $(basename "$runner") -> $dest_dir/"

# Copy companion native libraries that sit next to the runner in target/.
shopt -s nullglob
for lib in "$target_dir"/*.so "$target_dir"/*.so.* "$target_dir"/*.dll "$target_dir"/*.jnilib "$target_dir"/*.dylib; do
  cp "$lib" "$dest_dir/"
  echo "copied $(basename "$lib")"
done
shopt -u nullglob

echo "backend assembled into $dest_dir/"
