#!/bin/bash
#
# Sigil Daemon Startup Enablement Script
#
# Enables/disables the Sigil MPC signing daemon to start automatically
# on Linux boot using systemd.
#
# Usage:
#   sudo ./enable-daemon.sh           # Enable and start the daemon
#   sudo ./enable-daemon.sh --disable # Disable and stop the daemon
#   sudo ./enable-daemon.sh --status  # Check daemon status
#
# Prerequisites:
#   - sigil-daemon binary installed at /usr/local/bin/sigil-daemon
#   - systemd-based Linux distribution
#   - Run as root (sudo)
#

set -euo pipefail

# Configuration
DAEMON_BINARY="/usr/local/bin/sigil-daemon"
SERVICE_NAME="sigil-daemon"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
UDEV_RULES_FILE="/etc/udev/rules.d/99-sigil.rules"
CONFIG_DIR="/etc/sigil"
DATA_DIR="/var/lib/sigil"
SIGIL_GROUP="sigil"

# Script directory (for finding source files)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_SERVICE_FILE="${SCRIPT_DIR}/udev-rules/sigil-daemon.service"
SOURCE_UDEV_RULES="${SCRIPT_DIR}/udev-rules/99-sigil.rules"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Output functions
info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

die() {
    error "$1"
    exit 1
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        die "This script must be run as root (use sudo)"
    fi
}

# Check if systemd is available
check_systemd() {
    if ! command -v systemctl &> /dev/null; then
        die "systemctl not found - this script requires systemd"
    fi
    if ! pidof systemd &> /dev/null; then
        die "systemd is not running"
    fi
}

# Check if daemon binary exists
check_binary() {
    if [[ ! -x "$DAEMON_BINARY" ]]; then
        die "sigil-daemon binary not found at $DAEMON_BINARY
Please run install.sh first to build and install the daemon"
    fi
}

# Check if source files exist
check_source_files() {
    if [[ ! -f "$SOURCE_SERVICE_FILE" ]]; then
        die "Service file not found at $SOURCE_SERVICE_FILE"
    fi
    if [[ ! -f "$SOURCE_UDEV_RULES" ]]; then
        die "udev rules file not found at $SOURCE_UDEV_RULES"
    fi
}

# Create sigil group and add user to groups
setup_groups() {
    info "Setting up groups..."

    # Create sigil group if it doesn't exist
    if ! getent group "$SIGIL_GROUP" &> /dev/null; then
        groupadd "$SIGIL_GROUP"
        success "Created group: $SIGIL_GROUP"
    else
        success "Group already exists: $SIGIL_GROUP"
    fi

    # Add the invoking user to sigil and plugdev groups
    # SUDO_USER contains the original user who invoked sudo
    if [[ -n "${SUDO_USER:-}" ]]; then
        if ! groups "$SUDO_USER" | grep -q "\b${SIGIL_GROUP}\b"; then
            usermod -aG "$SIGIL_GROUP" "$SUDO_USER"
            success "Added $SUDO_USER to $SIGIL_GROUP group"
        else
            success "$SUDO_USER already in $SIGIL_GROUP group"
        fi

        if ! groups "$SUDO_USER" | grep -q "\bplugdev\b"; then
            # plugdev group may not exist on all systems
            if getent group plugdev &> /dev/null; then
                usermod -aG plugdev "$SUDO_USER"
                success "Added $SUDO_USER to plugdev group"
            else
                warn "plugdev group does not exist (optional)"
            fi
        else
            success "$SUDO_USER already in plugdev group"
        fi
    else
        warn "Could not determine invoking user (SUDO_USER not set)"
    fi
}

# Create necessary directories
setup_directories() {
    info "Setting up directories..."

    # Config directory
    if [[ ! -d "$CONFIG_DIR" ]]; then
        mkdir -p "$CONFIG_DIR"
        chmod 755 "$CONFIG_DIR"
        success "Created config directory: $CONFIG_DIR"
    else
        success "Config directory exists: $CONFIG_DIR"
    fi

    # Data directory
    if [[ ! -d "$DATA_DIR" ]]; then
        mkdir -p "$DATA_DIR"
        chmod 755 "$DATA_DIR"
        success "Created data directory: $DATA_DIR"
    else
        success "Data directory exists: $DATA_DIR"
    fi
}

# Install systemd service file
install_service() {
    info "Installing systemd service..."

    # Copy service file
    cp "$SOURCE_SERVICE_FILE" "$SERVICE_FILE"
    chmod 644 "$SERVICE_FILE"
    success "Installed service file: $SERVICE_FILE"
}

# Install udev rules
install_udev_rules() {
    info "Installing udev rules..."

    # Copy udev rules
    cp "$SOURCE_UDEV_RULES" "$UDEV_RULES_FILE"
    chmod 644 "$UDEV_RULES_FILE"
    success "Installed udev rules: $UDEV_RULES_FILE"

    # Reload udev rules
    udevadm control --reload-rules
    udevadm trigger
    success "Reloaded udev rules"
}

# Enable and start the daemon
enable_daemon() {
    info "Enabling and starting daemon..."

    # Reload systemd to pick up new service file
    systemctl daemon-reload
    success "Reloaded systemd configuration"

    # Enable the service (start on boot)
    systemctl enable "$SERVICE_NAME"
    success "Enabled $SERVICE_NAME to start on boot"

    # Start the service now
    if systemctl is-active --quiet "$SERVICE_NAME"; then
        systemctl restart "$SERVICE_NAME"
        success "Restarted $SERVICE_NAME"
    else
        systemctl start "$SERVICE_NAME"
        success "Started $SERVICE_NAME"
    fi
}

# Disable and stop the daemon
disable_daemon() {
    info "Disabling and stopping daemon..."

    # Stop the service if running
    if systemctl is-active --quiet "$SERVICE_NAME"; then
        systemctl stop "$SERVICE_NAME"
        success "Stopped $SERVICE_NAME"
    else
        success "$SERVICE_NAME was not running"
    fi

    # Disable the service (don't start on boot)
    if systemctl is-enabled --quiet "$SERVICE_NAME" 2>/dev/null; then
        systemctl disable "$SERVICE_NAME"
        success "Disabled $SERVICE_NAME from starting on boot"
    else
        success "$SERVICE_NAME was not enabled"
    fi

    echo ""
    info "Note: Service file and udev rules were NOT removed."
    info "To completely uninstall, manually remove:"
    info "  - $SERVICE_FILE"
    info "  - $UDEV_RULES_FILE"
}

# Verify the daemon is running correctly
verify_daemon() {
    info "Verifying daemon status..."
    echo ""

    if systemctl is-active --quiet "$SERVICE_NAME"; then
        success "Daemon is running"
        echo ""
        systemctl status "$SERVICE_NAME" --no-pager -l || true
    else
        error "Daemon is not running"
        echo ""
        echo "Recent logs:"
        journalctl -u "$SERVICE_NAME" -n 20 --no-pager || true
        return 1
    fi
}

# Show status of the daemon
show_status() {
    echo -e "${BLUE}=== Sigil Daemon Status ===${NC}"
    echo ""

    # Check binary
    if [[ -x "$DAEMON_BINARY" ]]; then
        success "Binary installed: $DAEMON_BINARY"
    else
        error "Binary not found: $DAEMON_BINARY"
    fi

    # Check service file
    if [[ -f "$SERVICE_FILE" ]]; then
        success "Service file installed: $SERVICE_FILE"
    else
        warn "Service file not installed: $SERVICE_FILE"
    fi

    # Check udev rules
    if [[ -f "$UDEV_RULES_FILE" ]]; then
        success "udev rules installed: $UDEV_RULES_FILE"
    else
        warn "udev rules not installed: $UDEV_RULES_FILE"
    fi

    # Check group
    if getent group "$SIGIL_GROUP" &> /dev/null; then
        success "Group exists: $SIGIL_GROUP"
    else
        warn "Group does not exist: $SIGIL_GROUP"
    fi

    # Check directories
    if [[ -d "$CONFIG_DIR" ]]; then
        success "Config directory exists: $CONFIG_DIR"
    else
        warn "Config directory missing: $CONFIG_DIR"
    fi

    if [[ -d "$DATA_DIR" ]]; then
        success "Data directory exists: $DATA_DIR"
    else
        warn "Data directory missing: $DATA_DIR"
    fi

    echo ""

    # Service status
    if systemctl is-enabled --quiet "$SERVICE_NAME" 2>/dev/null; then
        success "Service enabled (starts on boot)"
    else
        warn "Service not enabled (won't start on boot)"
    fi

    if systemctl is-active --quiet "$SERVICE_NAME"; then
        success "Service is running"
        echo ""
        systemctl status "$SERVICE_NAME" --no-pager -l 2>/dev/null || true
    else
        warn "Service is not running"
    fi
}

# Print usage
usage() {
    echo "Sigil Daemon Startup Enablement Script"
    echo ""
    echo "Usage: sudo $0 [OPTION]"
    echo ""
    echo "Options:"
    echo "  (none)     Enable and start the daemon (default)"
    echo "  --disable  Disable and stop the daemon"
    echo "  --status   Show daemon status"
    echo "  --help     Show this help message"
    echo ""
    echo "Examples:"
    echo "  sudo $0           # Enable and start the daemon"
    echo "  sudo $0 --status  # Check current status"
    echo "  sudo $0 --disable # Disable the daemon"
}

# Main function
main() {
    local action="enable"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --disable)
                action="disable"
                shift
                ;;
            --status)
                action="status"
                shift
                ;;
            --help|-h)
                usage
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done

    echo -e "${BLUE}=== Sigil Daemon Setup ===${NC}"
    echo ""

    case $action in
        enable)
            check_root
            check_systemd
            check_binary
            check_source_files

            echo ""
            setup_groups
            echo ""
            setup_directories
            echo ""
            install_service
            echo ""
            install_udev_rules
            echo ""
            enable_daemon
            echo ""
            verify_daemon

            echo ""
            echo -e "${GREEN}=== Setup Complete ===${NC}"
            echo ""
            info "The Sigil daemon is now enabled and will start automatically on boot."
            info "Use 'journalctl -u sigil-daemon -f' to view live logs."

            if [[ -n "${SUDO_USER:-}" ]]; then
                echo ""
                warn "You may need to log out and back in for group changes to take effect."
            fi
            ;;
        disable)
            check_root
            check_systemd

            echo ""
            disable_daemon

            echo ""
            echo -e "${GREEN}=== Disable Complete ===${NC}"
            ;;
        status)
            show_status
            ;;
    esac
}

main "$@"
