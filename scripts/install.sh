#!/bin/bash
# Sigil Installation Script
#
# One-liner install:
#   curl -sSL https://raw.githubusercontent.com/chippr-robotics/sigil/main/scripts/install.sh | sudo bash
#
# Or clone and run locally:
#   sudo ./scripts/install.sh

set -e

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
    # Check if we're in the sigil repo already
    if [[ -f "Cargo.toml" ]] && grep -q 'name = "sigil"' Cargo.toml 2>/dev/null; then
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
    cargo build --release --quiet \
        -p sigil-daemon \
        -p sigil-cli \
        -p sigil-mother \
        -p sigil-frost --all-features \
        2>&1 | grep -v "Compiling\|Downloading" || true

    info "Build complete"
}

# Install binaries
install_binaries() {
    step "Installing binaries to $INSTALL_DIR..."
    mkdir -p "$INSTALL_DIR"

    for bin in sigil-daemon sigil sigil-mother; do
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
    "ipc_socket_path": "/tmp/sigil.sock",
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
    else
        echo "Add to your PATH:"
        echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
        echo ""
        echo "Then run:"
        echo "  sigil-daemon &"
        echo "  sigil status"
    fi
    echo ""
}

# Uninstall
uninstall() {
    check_environment
    step "Uninstalling sigil..."

    rm -f "$INSTALL_DIR"/{sigil-daemon,sigil,sigil-mother}

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
Sigil Installation Script

Usage: install.sh [OPTIONS]

Options:
  -h, --help      Show this help
  --uninstall     Remove sigil

Environment:
  INSTALL_DIR     Binary location (default: /usr/local/bin)
  CONFIG_DIR      Config location (default: /etc/sigil)
  DATA_DIR        Data location (default: /var/lib/sigil)

One-liner install:
  curl -sSL https://raw.githubusercontent.com/chippr-robotics/sigil/main/scripts/install.sh | sudo bash
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
