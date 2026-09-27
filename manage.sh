#!/usr/bin/env bash
set -euo pipefail

CARGO="${HOME}/.cargo/bin/cargo"
BINARY="target/release/imt"
INSTALL_DIR="${HOME}/.local/bin"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cd "$PROJECT_DIR"

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')

usage() {
    cat <<EOF
Indicium Mail TUI v${VERSION} - manage.sh

Usage: ./manage.sh <command>

Commands:
  build         Build debug binary (fast, unoptimised)
  release       Build optimised release binary
  run           Build release and run  imt run
  dev           Build debug and run    imt run
  install       Install release binary to ${INSTALL_DIR}
  uninstall     Remove installed binary
  check         Run cargo check (fast syntax/type check)
  test          Run cargo test (workspace; OPENSSL_* if no pkg-config)
  lint          Run clippy linter
  fmt           Format all source files
  clean         Remove build artefacts
  version       Show current workspace version
  bump <part>   Bump version: major | minor | patch
  help          Show this help

Extra args after test are passed to cargo test, e.g.:
  ./manage.sh test -p imt-tui --lib
EOF
}

need_cargo() {
    if [[ ! -x "$CARGO" ]]; then
        echo "error: cargo not found at $CARGO" >&2
        exit 1
    fi
}

# openssl-sys needs pkg-config or explicit paths (see TESTS.md / BUILD.md).
ensure_openssl_env() {
    if command -v pkg-config >/dev/null 2>&1; then
        return 0
    fi
    export OPENSSL_DIR="${OPENSSL_DIR:-/usr}"
    export OPENSSL_LIB_DIR="${OPENSSL_LIB_DIR:-/usr/lib/x86_64-linux-gnu}"
    export OPENSSL_INCLUDE_DIR="${OPENSSL_INCLUDE_DIR:-/usr/include}"
}

cmd_build() {
    need_cargo
    echo "Building debug..."
    "$CARGO" build
    echo "Binary: target/debug/imt"
}

cmd_release() {
    need_cargo
    echo "Building release..."
    "$CARGO" build --release
    echo "Binary: ${BINARY}"
}

cmd_run() {
    cmd_release
    echo ""
    exec "${BINARY}" run
}

cmd_dev() {
    need_cargo
    echo "Building debug..."
    "$CARGO" build
    echo ""
    exec target/debug/imt run
}

cmd_install() {
    cmd_release
    mkdir -p "$INSTALL_DIR"
    cp "$BINARY" "${INSTALL_DIR}/imt"
    echo "Installed: ${INSTALL_DIR}/imt"
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        echo "Note: ${INSTALL_DIR} is not in your PATH - add it to your shell profile."
    fi
}

cmd_uninstall() {
    local target="${INSTALL_DIR}/imt"
    if [[ -f "$target" ]]; then
        rm "$target"
        echo "Removed: $target"
    else
        echo "Not installed at $target"
    fi
}

cmd_check() {
    need_cargo
    ensure_openssl_env
    echo "Checking..."
    "$CARGO" check
}

cmd_test() {
    need_cargo
    ensure_openssl_env
    echo "Testing..."
    "$CARGO" test --workspace "$@"
}

cmd_lint() {
    need_cargo
    ensure_openssl_env
    echo "Running clippy..."
    "$CARGO" clippy -- -D warnings
}

cmd_fmt() {
    need_cargo
    echo "Formatting..."
    "$CARGO" fmt
}

cmd_clean() {
    need_cargo
    echo "Cleaning build artefacts..."
    "$CARGO" clean
    echo "Done."
}

cmd_version() {
    echo "v${VERSION}"
}

bump_version() {
    local part="${1:-patch}"
    local major minor patch
    IFS='.' read -r major minor patch <<< "$VERSION"
    case "$part" in
        major) major=$((major + 1)); minor=0; patch=0 ;;
        minor) minor=$((minor + 1)); patch=0 ;;
        patch) patch=$((patch + 1)) ;;
        *) echo "error: part must be major, minor, or patch" >&2; exit 1 ;;
    esac
    local new_version="${major}.${minor}.${patch}"
    # Update workspace Cargo.toml
    sed -i "s/^version = \"${VERSION}\"/version = \"${new_version}\"/" Cargo.toml
    echo "Bumped: ${VERSION} -> ${new_version}"
    echo "Remember to update CHANGELOG.md and commit."
}

case "${1:-help}" in
    build)     cmd_build ;;
    release)   cmd_release ;;
    run)       cmd_run ;;
    dev)       cmd_dev ;;
    install)   cmd_install ;;
    uninstall) cmd_uninstall ;;
    check)     cmd_check ;;
    test)      shift; cmd_test "$@" ;;
    lint)      cmd_lint ;;
    fmt)       cmd_fmt ;;
    clean)     cmd_clean ;;
    version)   cmd_version ;;
    bump)      bump_version "${2:-patch}" ;;
    help|--help|-h) usage ;;
    *) echo "Unknown command: ${1}"; echo; usage; exit 1 ;;
esac
