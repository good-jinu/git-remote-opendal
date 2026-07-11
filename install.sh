#!/bin/sh
# Copyright the git-opendal authors. MIT license.
# Install the OpenDAL Git extensions: the remote helper and workflow command.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/good-jinu/git-opendal/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/good-jinu/git-opendal/main/install.sh | sh -s -- --version v0.1.0

set -e

PRODUCT_NAME="git-opendal"
BINARY_NAME="git-remote-opendal"
WORKFLOW_BINARY="git-opendal"
GITHUB_REPO="good-jinu/git-opendal"
INSTALL_DIR="${GIT_OPENDAL_INSTALL:-${GIT_REMOTE_OPENDAL_INSTALL:-$HOME/.git-opendal}}"
BIN_DIR="$INSTALL_DIR/bin"
EXE="$BIN_DIR/$BINARY_NAME"
WORKFLOW_EXE="$BIN_DIR/$WORKFLOW_BINARY"

# ── helpers ──────────────────────────────────────────────────────────────────

say()  { echo "  $*"; }
err()  { echo "error: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || err "'$1' is required but not found on PATH."; }

print_help_and_exit() {
    cat <<EOF
Install script for $PRODUCT_NAME

USAGE:
    install.sh [OPTIONS] [VERSION]

OPTIONS:
    -y, --yes             Skip interactive prompts and add to PATH automatically
    --no-modify-path      Do not modify shell profile to add the Git extensions to PATH
    -h, --help            Print this help message

ARGS:
    VERSION               Tag to install (e.g. v0.3.0). Defaults to latest release.

ENVIRONMENT:
    GIT_OPENDAL_INSTALL          Override install directory (default: ~/.git-opendal)
    GIT_REMOTE_OPENDAL_INSTALL   Deprecated alias for GIT_OPENDAL_INSTALL

EOF
    echo "$BINARY_NAME was NOT installed."
    exit 0
}

# ── detect platform ──────────────────────────────────────────────────────────

detect_target() {
    if [ "${OS:-}" = "Windows_NT" ]; then
        echo "x86_64-pc-windows-msvc"
        return
    fi

    _os=$(uname -s)
    _arch=$(uname -m)

    case "$_os" in
        Darwin)
            case "$_arch" in
                arm64)   echo "aarch64-apple-darwin" ;;
                x86_64)  echo "x86_64-apple-darwin" ;;
                *)       err "Unsupported macOS architecture: $_arch" ;;
            esac
            ;;
        Linux)
            case "$_arch" in
                aarch64|arm64) echo "aarch64-unknown-linux-musl" ;;
                x86_64)        echo "x86_64-unknown-linux-musl" ;;
                *)             err "Unsupported Linux architecture: $_arch" ;;
            esac
            ;;
        *)
            err "Unsupported OS: $_os. Please install manually from https://github.com/$GITHUB_REPO/releases"
            ;;
    esac
}

# ── parse args ───────────────────────────────────────────────────────────────

version=""
modify_path=true
auto_yes=false

for arg in "$@"; do
    case "$arg" in
        -h|--help)            print_help_and_exit ;;
        -y|--yes)             auto_yes=true ;;
        --no-modify-path)     modify_path=false ;;
        -*)                   ;;   # ignore unknown flags
        *)
            if [ -z "$version" ]; then
                version="$arg"
            fi
            ;;
    esac
done

# ── resolve version ──────────────────────────────────────────────────────────

need curl

if [ -z "$version" ]; then
    say "Fetching latest release version..."
    version=$(curl -fsSL "https://api.github.com/repos/$GITHUB_REPO/releases/latest" \
        | grep '"tag_name"' \
        | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
    if [ -z "$version" ]; then
        err "Could not determine the latest release. Set a version explicitly: install.sh v0.1.0"
    fi
fi

say "Installing $BINARY_NAME and $WORKFLOW_BINARY $version"

# ── build download URL ───────────────────────────────────────────────────────

target=$(detect_target)

case "$target" in
    *windows*) ext="zip" ;;
    *)         ext="tar.gz" ;;
esac

archive="${PRODUCT_NAME}-${version}-${target}.${ext}"
url="https://github.com/$GITHUB_REPO/releases/download/$version/$archive"

say "Target : $target"
say "URL    : $url"

# ── download & extract ───────────────────────────────────────────────────────

mkdir -p "$BIN_DIR"
tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

say "Downloading..."
curl --fail --location --progress-bar --output "$tmp_dir/$archive" "$url"

say "Extracting..."
case "$ext" in
    tar.gz)
        need tar
        tar -xzf "$tmp_dir/$archive" -C "$tmp_dir"
        # The archive contains a single top-level directory
        extracted=$(find "$tmp_dir" -maxdepth 1 -mindepth 1 -type d | head -n 1)
        mv "$extracted/$BINARY_NAME" "$EXE"
        mv "$extracted/$WORKFLOW_BINARY" "$BIN_DIR/$WORKFLOW_BINARY"
        ;;
    zip)
        if command -v unzip >/dev/null 2>&1; then
            unzip -q "$tmp_dir/$archive" -d "$tmp_dir"
        elif command -v 7z >/dev/null 2>&1; then
            7z x -o"$tmp_dir" -y "$tmp_dir/$archive" >/dev/null
        else
            err "Either 'unzip' or '7z' is required to extract the archive."
        fi
        extracted=$(find "$tmp_dir" -maxdepth 1 -mindepth 1 -type d | head -n 1)
        mv "$extracted/${BINARY_NAME}.exe" "${EXE}.exe"
        mv "$extracted/${WORKFLOW_BINARY}.exe" "${WORKFLOW_EXE}.exe"
        EXE="${EXE}.exe"
        WORKFLOW_EXE="${WORKFLOW_EXE}.exe"
        ;;
esac

chmod +x "$EXE"
chmod +x "$WORKFLOW_EXE" 2>/dev/null || true
say "$BINARY_NAME installed to $EXE"
say "$WORKFLOW_BINARY installed to $WORKFLOW_EXE"

# ── verify ───────────────────────────────────────────────────────────────────

if "$WORKFLOW_EXE" --version >/dev/null 2>&1; then
    installed_version=$("$WORKFLOW_EXE" --version 2>&1 | head -n 1)
    say "Verified: $installed_version"
fi

# ── PATH setup ───────────────────────────────────────────────────────────────

add_to_path_line() {
    printf '\n# OpenDAL Git extensions\nexport PATH="%s:$PATH"\n' "$BIN_DIR"
}

if command -v "$BINARY_NAME" >/dev/null 2>&1 && command -v "$WORKFLOW_BINARY" >/dev/null 2>&1; then
    say "Both Git extensions are already on your PATH — no changes needed."
    modify_path=false
fi

if [ "$modify_path" = "true" ]; then
    # Detect shell profile
    profile=""
    if [ -n "${ZSH_VERSION:-}" ] || [ "$(basename "${SHELL:-}")" = "zsh" ]; then
        profile="$HOME/.zshrc"
    elif [ -n "${BASH_VERSION:-}" ] || [ "$(basename "${SHELL:-}")" = "bash" ]; then
        profile="${HOME}/.bashrc"
        [ -f "$HOME/.bash_profile" ] && profile="$HOME/.bash_profile"
    elif [ "$(basename "${SHELL:-}")" = "fish" ]; then
        profile="$HOME/.config/fish/config.fish"
    fi

    if [ -z "$profile" ]; then
        say "Could not detect your shell profile. Add the following to your profile manually:"
        say "  export PATH=\"$BIN_DIR:\$PATH\""
    elif $auto_yes; then
        add_to_path_line >> "$profile"
        say "Added $BIN_DIR to PATH in $profile"
    else
        printf "\nAdd %s to PATH in %s? [y/N] " "$BIN_DIR" "$profile"
        if [ -t 0 ]; then
            read -r _answer
        else
            # Running piped — read from tty directly
            read -r _answer </dev/tty
        fi
        case "$_answer" in
            y|Y|yes|Yes)
                add_to_path_line >> "$profile"
                say "Added $BIN_DIR to PATH in $profile"
                ;;
            *)
                say "Skipped. Add this to your shell profile manually:"
                say "  export PATH=\"$BIN_DIR:\$PATH\""
                ;;
        esac
    fi
fi

# ── done ─────────────────────────────────────────────────────────────────────

echo ""
echo "OpenDAL Git extensions were installed successfully!"
echo ""
if command -v "$BINARY_NAME" >/dev/null 2>&1 && command -v "$WORKFLOW_BINARY" >/dev/null 2>&1; then
    echo "Run 'git opendal --help' to get started."
else
    echo "Run '$WORKFLOW_EXE --help' to get started."
    echo "(You may need to restart your shell or run: export PATH=\"$BIN_DIR:\$PATH\")"
fi
echo ""
echo "Documentation: https://github.com/$GITHUB_REPO#readme"
