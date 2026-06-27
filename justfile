# Dev recipes for PCPanel. Run `just --list` to see available commands.

export JAVA_HOME := env("JAVA_HOME", env("HOME") / ".jdks/graalvm-community-openjdk-25.0.2+10.1/Contents/Home")

# Start the Quarkus backend and Tauri shell together. Ctrl-C kills both.
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    BACKEND_PID=""
    cleanup() {
        [[ -n "$BACKEND_PID" ]] && kill "$BACKEND_PID" 2>/dev/null && wait "$BACKEND_PID" 2>/dev/null
        # Kill any orphaned Quarkus/Java processes on our dev ports.
        lsof -ti:7654 | xargs kill 2>/dev/null || true
    }
    trap cleanup EXIT INT TERM
    ./mvnw compile -q
    ./mvnw quarkus:dev -Dquarkus.console.enabled=false -Ddebug=false &
    BACKEND_PID=$!
    echo "Waiting for backend on :7654..."
    for i in $(seq 1 120); do
        if curl -sf http://127.0.0.1:7654 >/dev/null 2>&1; then
            echo "Backend is up."
            break
        fi
        sleep 0.5
    done
    cd src-tauri && cargo tauri dev

# Start the Quarkus backend and Tauri shell with the MCP dev server enabled.
dev-mcp:
    #!/usr/bin/env bash
    set -euo pipefail
    BACKEND_PID=""
    cleanup() {
        [[ -n "$BACKEND_PID" ]] && kill "$BACKEND_PID" 2>/dev/null && wait "$BACKEND_PID" 2>/dev/null
        lsof -ti:7654 | xargs kill 2>/dev/null || true
    }
    trap cleanup EXIT INT TERM
    ./mvnw compile -q
    ./mvnw quarkus:dev -Dquarkus.console.enabled=false -Ddebug=false -Dpcpanel.mcp=true &
    BACKEND_PID=$!
    echo "Waiting for backend on :7654..."
    for i in $(seq 1 120); do
        if curl -sf http://127.0.0.1:7654 >/dev/null 2>&1; then
            echo "Backend is up."
            break
        fi
        sleep 0.5
    done
    cd src-tauri && cargo tauri dev

# Start only the Quarkus backend in dev mode.
backend:
    ./mvnw quarkus:dev

# Start only the Tauri shell (assumes the backend is already running).
shell:
    cd src-tauri && cargo tauri dev

# Run Java unit tests.
test:
    ./mvnw test

# Run a single Java test class or method (e.g. just test-one SaveTest#testMigration).
test-one TEST:
    ./mvnw test -Dtest={{TEST}}

# Run Angular frontend tests.
test-ui:
    cd src/main/webui && npm test

# Build JVM-only jar (fast, no GraalVM needed).
build:
    ./mvnw clean package -Dquarkus.native.enabled=false

# Build native image (requires GraalVM).
build-native:
    ./mvnw clean package -Pnative

# Build the full packaged Tauri app (native image + shell installer).
package:
    #!/usr/bin/env bash
    set -euo pipefail
    ./mvnw clean package -Pnative
    packaging/assemble-backend.sh
    cd src-tauri && cargo tauri build

# Assemble the native backend into src-tauri/backend/ (run after build-native).
assemble:
    packaging/assemble-backend.sh

# Regenerate TypeScript types from Java DTOs.
generate-types:
    ./mvnw compile -pl .

# Regenerate Tauri icons from app-icon.png.
icons:
    cd src-tauri && cargo tauri icon ../app-icon.png

# Bump the project version (e.g. just bump 2.1).
bump VERSION:
    packaging/bump-version.sh {{VERSION}}

# Smoke-test the native binary (e.g. just smoke target/pcpanel-runner).
smoke BINARY:
    packaging/smoke-test.sh {{BINARY}}

# Install frontend dependencies.
npm-install:
    cd src/main/webui && npm install
