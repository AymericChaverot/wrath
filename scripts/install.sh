#!/bin/sh
# Wrath CLI installer for macOS and Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/AymericChaverot/wrath/main/scripts/install.sh | sh

set -e

REPO="AymericChaverot/wrath"
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="wrath"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() {
    printf "${GREEN}[*]${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}[!]${NC} %s\n" "$1"
}

error() {
    printf "${RED}[x]${NC} %s\n" "$1"
    exit 1
}

# Detect OS and architecture
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux)  OS_TARGET="unknown-linux-gnu" ;;
        Darwin) OS_TARGET="apple-darwin" ;;
        *)      error "Unsupported operating system: $OS" ;;
    esac

    case "$ARCH" in
        x86_64|amd64)   ARCH_TARGET="x86_64" ;;
        aarch64|arm64)  ARCH_TARGET="aarch64" ;;
        *)              error "Unsupported architecture: $ARCH" ;;
    esac

    TARGET="${ARCH_TARGET}-${OS_TARGET}"
    info "Detected platform: $TARGET"
}

# Fetch the latest release version
fetch_latest_version() {
    LATEST_VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')

    if [ -z "$LATEST_VERSION" ]; then
        error "Failed to fetch latest version"
    fi

    info "Latest version: $LATEST_VERSION"
}

# Download and install
install() {
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${LATEST_VERSION}/wrath-${TARGET}.tar.gz"
    TEMP_DIR=$(mktemp -d)

    info "Downloading from $DOWNLOAD_URL"
    curl -fsSL "$DOWNLOAD_URL" -o "${TEMP_DIR}/wrath.tar.gz" || error "Download failed. Check if a release exists for your platform: $TARGET"

    info "Extracting..."
    tar -xzf "${TEMP_DIR}/wrath.tar.gz" -C "$TEMP_DIR"

    info "Installing to ${INSTALL_DIR}/${BINARY_NAME}"
    if [ -w "$INSTALL_DIR" ]; then
        mv "${TEMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    else
        warn "Elevated permissions required to install to $INSTALL_DIR"
        sudo mv "${TEMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    fi

    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    rm -rf "$TEMP_DIR"

    info "Installed ${BINARY_NAME} ${LATEST_VERSION} to ${INSTALL_DIR}/${BINARY_NAME}"
}

# Verify installation
verify() {
    if command -v "$BINARY_NAME" >/dev/null 2>&1; then
        info "Verification: $($BINARY_NAME --version)"
        printf "\n${GREEN}Installation complete.${NC} Run '${BINARY_NAME} --help' to get started.\n"
    else
        warn "Binary installed but not found in PATH. Add ${INSTALL_DIR} to your PATH."
    fi
}

main() {
    printf "\n"
    printf "  ╦ ╦╦═╗╔═╗╔╦╗╦ ╦\n"
    printf "  ║║║╠╦╝╠═╣ ║ ╠═╣\n"
    printf "  ╚╩╝╩╚═╩ ╩ ╩ ╩ ╩\n"
    printf "  Installer\n\n"

    detect_platform
    fetch_latest_version
    install
    verify
}

main
