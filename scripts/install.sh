#!/bin/bash
# Sigil System Integration Script
#
# This script does NOT install Sigil's binaries. The supported install is:
#
#   cargo install --locked --git https://github.com/chippr-robotics/sigil --tag <version> \
#       sigil-daemon sigil-cli sigil-mother sigil-mother-tui sigil-mcp
#
# What this script does is the part cargo cannot: create the `sigil` group,
# install udev rules for disk and Ledger detection, write the daemon config,
# and install the systemd unit.
#
# Run it from a checkout you have read:
#
#   git clone https://github.com/chippr-robotics/sigil
#   cd sigil
#   less scripts/install.sh      # read it first
#   sudo ./scripts/install.sh
#
# It deliberately REFUSES to run when piped from the network. Sigil is a key
# custody product; asking an operator to execute unread, unpinned code as root
# is the wrong first instruction. See .specify/memory/constitution.md,
# Principle V.

set -e

# Refuse `curl ... | sudo bash`.
#
# When a script is piped into bash, $0 is "bash" (or the shell's name) and
# BASH_SOURCE[0] either matches it or points at a pipe rather than a regular
# file. In that case there is nothing on disk for the operator to have read.
refuse_pipe_execution() {
    local source="${BASH_SOURCE[0]:-}"

    if [[ -n "$source" && -f "$source" && "$source" != "$0" ]]; then
        return 0   # sourced from a real file
    fi
    if [[ -n "$source" && -f "$source" ]]; then
        return 0   # executed as a real file
    fi

    cat >&2 <<'REFUSE'
[ERROR] Refusing to run from a pipe.

This script was piped into a shell, so you have not read what is about to run
as root on the machine that will hold your signing shards.

Do this instead:

    git clone https://github.com/chippr-robotics/sigil
    cd sigil
    less scripts/install.sh
    sudo ./scripts/install.sh

And install the binaries themselves with cargo, pinned to a release tag:

    cargo install --locked --git https://github.com/chippr-robotics/sigil \
        --tag <version> sigil-daemon sigil-cli sigil-mother sigil-mcp
REFUSE
    exit 1
}

refuse_pipe_execution

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
CONFIG_DIR="${CONFIG_DIR:-/etc/sigil}"
DATA_DIR="${DATA_DIR:-/var/lib/sigil}"
SIGIL_GROUP="sigil"
REPO_URL="https://github.com/chippr-robotics/sigil.git"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC} $1"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }
step()  { echo -e "${BLUE}==>${NC} $1"; }

# Detect environment
check_environment() {
    if [[ $EUID -ne 0 ]]; then
        warn "Not running as root. Installing to user directories."
        INSTALL_DIR="$HOME/.local/bin"
        CONFIG_DIR="$HOME/.config/sigil"
        DATA_DIR="$HOME/.local/share/sigil"
        SYSTEM_INSTALL=false
    else
        SYSTEM_INSTALL=true
    fi

    # Detect OS
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        OS="linux"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        OS="macos"
    else
        OS="unknown"
        warn "Unknown OS: $OSTYPE"
    fi
}

# Check and install Rust if needed
ensure_rust() {
    if ! command -v cargo &> /dev/null; then
        step "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
        source "$HOME/.cargo/env"
    fi
    info "Rust $(rustc --version | cut -d' ' -f2) found"
}

# Install system dependencies
install_deps() {
    if [[ "$OS" != "linux" ]]; then
        return
    fi

    step "Installing system dependencies..."

    if [[ "$SYSTEM_INSTALL" != true ]]; then
        warn "Skipping system deps (not root). You may need: libudev-dev pkg-config libssl-dev"
        return
    fi

    if command -v apt-get &> /dev/null; then
        apt-get update -qq
        apt-get install -y -qq libudev-dev pkg-config libssl-dev build-essential git
    elif command -v dnf &> /dev/null; then
        dnf install -y -q systemd-devel openssl-devel gcc git
    elif command -v pacman &> /dev/null; then
        pacman -Sy --noconfirm --quiet systemd openssl base-devel git
    elif command -v apk &> /dev/null; then
        apk add --quiet eudev-dev openssl-dev build-base git
    fi
}

# Get source code
get_source() {
    # Check if we're in the sigil repo already (workspace or crate)
    if [[ -f "Cargo.toml" ]] && (grep -q 'sigil-core' Cargo.toml 2>/dev/null || grep -q 'name = "sigil"' Cargo.toml 2>/dev/null); then
        BUILD_DIR="$(pwd)"
        info "Building from current directory"
        return
    fi

    step "Downloading sigil source..."
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT

    git clone --depth 1 --quiet "$REPO_URL" "$TEMP_DIR/sigil"
    BUILD_DIR="$TEMP_DIR/sigil"
}

# Build sigil
build_sigil() {
    step "Building sigil (this may take a few minutes)..."
    cd "$BUILD_DIR"

    # Build all sigil components:
    # - sigil-daemon: Background signing daemon
    # - sigil-cli: CLI for signing operations
    # - sigil-mother: Air-gapped mother device tools
    # - sigil-frost: FROST threshold signature support (all curves)
    # - sigil-mcp: MCP server for universal agent integration
    cargo build --release --quiet \
        -p sigil-daemon \
        -p sigil-cli \
        -p sigil-mother --features sigil-mother/ledger \
        -p sigil-frost --all-features \
        -p sigil-mcp \
        2>&1 | grep -v "Compiling\|Downloading" || true

    info "Build complete"
}

# Install binaries
install_binaries() {
    step "Installing binaries to $INSTALL_DIR..."
    mkdir -p "$INSTALL_DIR"

    for bin in sigil-daemon sigil sigil-mother sigil-mcp; do
        if [[ -f "$BUILD_DIR/target/release/$bin" ]]; then
            cp "$BUILD_DIR/target/release/$bin" "$INSTALL_DIR/"
            chmod 755 "$INSTALL_DIR/$bin"
            info "Installed $bin"
        fi
    done
}

# Setup directories and config
setup_config() {
    step "Setting up configuration..."
    mkdir -p "$CONFIG_DIR" "$DATA_DIR"

    if [[ ! -f "$CONFIG_DIR/daemon.json" ]]; then
        cat > "$CONFIG_DIR/daemon.json" << EOF
{
    "agent_store_path": "$DATA_DIR/agent_store",
    "ipc_socket_path": "/run/sigil/sigil.sock",
    "ipc_socket_mode": 432,
    "enable_zkvm_proving": false,
    "disk_mount_pattern": "/media/*/SIGIL*",
    "signing_timeout_secs": 60,
    "dev_mode": false
}
EOF
        info "Created $CONFIG_DIR/daemon.json"
    fi
}

# Setup Linux-specific items (group, udev, systemd)
setup_linux() {
    [[ "$OS" != "linux" || "$SYSTEM_INSTALL" != true ]] && return

    step "Configuring Linux system..."

    # Create group
    if ! getent group "$SIGIL_GROUP" &>/dev/null; then
        groupadd "$SIGIL_GROUP"
        info "Created group: $SIGIL_GROUP"
    fi

    # Add user to group
    if [[ -n "$SUDO_USER" && "$SUDO_USER" != "root" ]]; then
        usermod -aG "$SIGIL_GROUP" "$SUDO_USER"
        usermod -aG plugdev "$SUDO_USER" 2>/dev/null || true
        info "Added $SUDO_USER to $SIGIL_GROUP group"
    fi

    # Set permissions
    chown -R root:"$SIGIL_GROUP" "$DATA_DIR"
    chmod 770 "$DATA_DIR"

    # Udev rules for Sigil disks
    cat > /etc/udev/rules.d/99-sigil.rules << 'EOF'
# Sigil floppy disk detection
ACTION=="add", SUBSYSTEM=="block", ENV{ID_TYPE}=="disk", ENV{ID_BUS}=="usb", TAG+="systemd"
ACTION=="remove", SUBSYSTEM=="block", ENV{ID_TYPE}=="disk", ENV{ID_BUS}=="usb", TAG+="systemd"
EOF

    # Udev rules for Ledger
    cat > /etc/udev/rules.d/20-ledger.rules << 'EOF'
# Ledger Nano S
SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", ATTRS{idProduct}=="0001", MODE="0660", GROUP="plugdev"
SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", ATTRS{idProduct}=="1011", MODE="0660", GROUP="plugdev"
# Ledger Nano X
SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", ATTRS{idProduct}=="0004", MODE="0660", GROUP="plugdev"
SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", ATTRS{idProduct}=="4011", MODE="0660", GROUP="plugdev"
EOF

    udevadm control --reload-rules
    udevadm trigger
    info "Installed udev rules"

    # Systemd service
    cat > /etc/systemd/system/sigil-daemon.service << EOF
[Unit]
Description=Sigil MPC Signing Daemon
After=network.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/sigil-daemon
Restart=on-failure
RestartSec=5
User=root
Group=$SIGIL_GROUP
Environment=RUST_LOG=info

# systemd creates /run/sigil before the daemon starts and removes it on stop.
# The IPC socket is an unauthenticated path to the signer, so it lives in a
# directory this unit owns rather than in world-writable /tmp.
RuntimeDirectory=sigil
RuntimeDirectoryMode=0750

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    info "Installed systemd service"
}

# Print completion message
print_complete() {
    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}   Sigil Installation Complete!${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "Installed components:"
    echo "  sigil-daemon  - Background signing daemon"
    echo "  sigil         - CLI for signing operations"
    echo "  sigil-mother  - Air-gapped mother device tools"
    echo "  sigil-mcp     - MCP server for AI agent integration"
    echo ""
    echo "Built libraries:"
    echo "  sigil-frost   - FROST threshold signatures"
    echo "                  (Taproot, Ed25519, Ristretto255)"
    echo ""

    if [[ "$SYSTEM_INSTALL" == true ]]; then
        echo "Quick start:"
        echo "  sudo systemctl enable --now sigil-daemon"
        echo "  sigil status"
        echo ""
        echo "Mother device (air-gapped):"
        echo "  sigil-mother init"
        echo "  sigil-mother init --ledger  # with Ledger hardware wallet"
        echo ""
        echo "FROST DKG ceremony:"
        echo "  sigil ceremony dkg-init --scheme taproot"
        echo ""
        echo "MCP server (for Claude Desktop, VS Code, etc.):"
        echo "  sigil-mcp --transport stdio"
    else
        echo "Add to your PATH:"
        echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
        echo ""
        echo "Then run:"
        echo "  sigil-daemon &"
        echo "  sigil status"
        echo ""
        echo "MCP server (for Claude Desktop, VS Code, etc.):"
        echo "  sigil-mcp --transport stdio"
    fi
    echo ""
}

# Uninstall
uninstall() {
    check_environment
    step "Uninstalling sigil..."

    rm -f "$INSTALL_DIR"/{sigil-daemon,sigil,sigil-mother,sigil-mcp}

    if [[ "$SYSTEM_INSTALL" == true && "$OS" == "linux" ]]; then
        systemctl stop sigil-daemon 2>/dev/null || true
        systemctl disable sigil-daemon 2>/dev/null || true
        rm -f /etc/systemd/system/sigil-daemon.service
        rm -f /etc/udev/rules.d/{99-sigil,20-ledger}.rules
        systemctl daemon-reload
        udevadm control --reload-rules
    fi

    info "Uninstalled. Config at $CONFIG_DIR preserved."
}

# Main
main() {
    echo ""
    echo -e "${BLUE}=== Sigil MPC Signing System ===${NC}"
    echo ""

    check_environment
    ensure_rust
    install_deps
    get_source
    build_sigil
    install_binaries
    setup_config
    setup_linux
    print_complete
}

# Handle arguments
case "${1:-}" in
    -h|--help)
        cat << 'HELP'
Sigil System Integration Script

Usage: sudo ./scripts/install.sh [OPTIONS]

Run from a checkout you have read. This script refuses to execute when piped
from the network.

Options:
  -h, --help      Show this help
  --uninstall     Remove sigil

Environment:
  INSTALL_DIR     Binary location (default: /usr/local/bin)
  CONFIG_DIR      Config location (default: /etc/sigil)
  DATA_DIR        Data location (default: /var/lib/sigil)

Supported install of the binaries themselves:
  cargo install --locked --git https://github.com/chippr-robotics/sigil \
      --tag <version> sigil-daemon sigil-cli sigil-mother sigil-mother-tui sigil-mcp
HELP
        exit 0
        ;;
    --uninstall)
        uninstall
        exit 0
        ;;
    *)
        main
        ;;
esac
