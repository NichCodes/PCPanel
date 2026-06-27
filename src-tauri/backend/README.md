# Bundled backend

This directory holds the **PCPanel Quarkus native-image distribution** that the Tauri shell spawns and
bundles as a resource. It is populated at package time (locally or in CI) and is **not** committed —
only this README and `.gitkeep` are tracked.

It must contain the native binary plus **all companion libraries** it loads from its own directory
(`SndCtrl.dll`, the jSerialComm native lib, `kdotool`, the `*.so`/`*.dll`/`*.jnilib` files, etc.) — the
native image is not a single self-contained file.

The binary must be named:

- `PCPanel.exe` on Windows
- `PCPanel` on macOS / Linux

## Populating it for a local native test

```bash
# 1. Build the backend native image (from the repo root)
./mvnw clean package -Pnative

# 2. Copy the runner + its companion libs here, renamed to PCPanel
#    (the *-runner(.exe) plus every *.dll / *.so next to it in target/)
#    e.g. on Linux:
cp target/*-runner          src-tauri/backend/PCPanel
cp target/*.so              src-tauri/backend/ 2>/dev/null || true

# 3. Build the shell
cd src-tauri && cargo tauri build
```

CI does the equivalent per-OS assembly before `tauri build` (see
`.github/workflows/build-and-release.yml`).
