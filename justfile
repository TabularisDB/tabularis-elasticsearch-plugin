set shell := ["bash", "-cu"]
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

# ---------------------------------------------------------------------------
# Cross-platform recipes (only shell-agnostic tooling — cargo, ppm).
# ---------------------------------------------------------------------------

# Build the plugin binary in debug mode (plus UI if present).
build: build-ui
    cargo build

# Build for release (what the GitHub Actions workflow ships).
release: build-ui
    cargo build --release

# Run unit tests.
test:
    cargo test

# Launch the local REPL that simulates Tabularis JSON-RPC calls over stdio.
repl:
    cargo run --bin test_plugin

# Run clippy on the workspace.
lint:
    cargo clippy --all-targets -- -D warnings

# Format the codebase.
fmt:
    cargo fmt --all

# ---------------------------------------------------------------------------
# Platform-specific recipes (file operations + plugin-dir conventions).
# ---------------------------------------------------------------------------

# Build the UI extension if present (no-op otherwise).
[unix]
build-ui:
    @if [ -f ui/package.json ]; then \
        echo "Building UI extension..."; \
        (cd ui && pnpm install && pnpm run build); \
    fi

[windows]
build-ui:
    if (Test-Path ui/package.json) {
        Write-Host "Building UI extension..."
        Push-Location ui
        try {
            pnpm i
            if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
            pnpm run build
            if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        } finally {
            Pop-Location
        }
    }

# Build + copy binary, manifest and (if present) UI bundle into Tabularis's plugin folder.
[linux]
dev-install: build
    mkdir -p ~/.local/share/tabularis/plugins/elasticsearch
    cp target/debug/elasticsearch-plugin ~/.local/share/tabularis/plugins/elasticsearch/
    cp manifest.json ~/.local/share/tabularis/plugins/elasticsearch/
    @if [ -f ui/dist/index.js ]; then \
        mkdir -p ~/.local/share/tabularis/plugins/elasticsearch/ui/dist; \
        cp ui/dist/index.js ~/.local/share/tabularis/plugins/elasticsearch/ui/dist/; \
    fi
    @echo "Installed to ~/.local/share/tabularis/plugins/elasticsearch"
    @echo "Restart Tabularis (or toggle the plugin in Settings) to pick up changes."

[macos]
dev-install: build
    mkdir -p "$HOME/Library/Application Support/com.debba.tabularis/plugins/elasticsearch"
    cp target/debug/elasticsearch-plugin "$HOME/Library/Application Support/com.debba.tabularis/plugins/elasticsearch/"
    cp manifest.json "$HOME/Library/Application Support/com.debba.tabularis/plugins/elasticsearch/"
    @if [ -f ui/dist/index.js ]; then \
        mkdir -p "$HOME/Library/Application Support/com.debba.tabularis/plugins/elasticsearch/ui/dist"; \
        cp ui/dist/index.js "$HOME/Library/Application Support/com.debba.tabularis/plugins/elasticsearch/ui/dist/"; \
    fi
    @echo "Installed to ~/Library/Application Support/com.debba.tabularis/plugins/elasticsearch"
    @echo "Restart Tabularis (or toggle the plugin in Settings) to pick up changes."

[windows]
dev-install: build
    $dest = Join-Path $env:APPDATA "debba.tabularis\plugins\elasticsearch"
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    Copy-Item "target\debug\elasticsearch-plugin.exe" $dest
    Copy-Item "manifest.json" $dest
    if (Test-Path "ui\dist\index.js") {
        New-Item -ItemType Directory -Force -Path (Join-Path $dest "ui\dist") | Out-Null
        Copy-Item "ui\dist\index.js" (Join-Path $dest "ui\dist")
    }
    Write-Host "Installed to $dest"
    Write-Host "Restart Tabularis (or toggle the plugin in Settings) to pick up changes."

# Remove the installed plugin.
[linux]
uninstall:
    rm -rf ~/.local/share/tabularis/plugins/elasticsearch

[macos]
uninstall:
    rm -rf "$HOME/Library/Application Support/com.debba.tabularis/plugins/elasticsearch"

[windows]
uninstall:
    $dest = Join-Path $env:APPDATA "com.debba.tabularis\plugins\elasticsearch"
    if (Test-Path $dest) { Remove-Item -Recurse -Force $dest }
