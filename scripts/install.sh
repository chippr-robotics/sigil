#!/bin/bash
# Sigil installation script for Linux
# Run with: sudo ./install.sh

set -e

echo "=== Sigil MPC Signing System Installation ==="
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Please run as root (sudo ./install.sh)"
    exit 1
fi

# Create sigil group if it doesn't exist
if ! getent group sigil > /dev/null 2>&1; then
    echo "Creating 'sigil' group..."
    groupadd sigil
fi

# Add current user to sigil group
SUDO_USER=${SUDO_USER:-$USER}
if [ "$SUDO_USER" != "root" ]; then
    echo "Adding user '$SUDO_USER' to 'sigil' group..."
    usermod -a -G sigil "$SUDO_USER"
fi

# Create directories
echo "Creating directories..."
mkdir -p /etc/sigil
mkdir -p /var/lib/sigil/agent_store
mkdir -p /var/log/sigil

# Set permissions
chown -R root:sigil /etc/sigil
chown -R root:sigil /var/lib/sigil
chmod 750 /etc/sigil
chmod 770 /var/lib/sigil
chmod 770 /var/lib/sigil/agent_store

# Install udev rules
echo "Installing udev rules..."
cp scripts/udev-rules/99-sigil.rules /etc/udev/rules.d/
udevadm control --reload-rules
udevadm trigger

# Install systemd service
echo "Installing systemd service..."
cp scripts/udev-rules/sigil-daemon.service /etc/systemd/system/
systemctl daemon-reload

# Create default config if it doesn't exist
if [ ! -f /etc/sigil/daemon.json ]; then
    echo "Creating default configuration..."
    cat > /etc/sigil/daemon.json << 'EOF'
{
    "agent_store_path": "/var/lib/sigil/agent_store",
    "ipc_socket_path": "/tmp/sigil.sock",
    "enable_zkvm_proving": false,
    "disk_mount_pattern": "/media/*/SIGIL*",
    "signing_timeout_secs": 60,
    "dev_mode": false
}
EOF
    chown root:sigil /etc/sigil/daemon.json
    chmod 640 /etc/sigil/daemon.json
fi

# Build and install binaries
echo "Building Sigil..."
cargo build --release

echo "Installing binaries..."
cp target/release/sigil-daemon /usr/local/bin/
cp target/release/sigil /usr/local/bin/
chmod 755 /usr/local/bin/sigil-daemon
chmod 755 /usr/local/bin/sigil

# Enable and start daemon
echo "Enabling sigil-daemon service..."
systemctl enable sigil-daemon

echo ""
echo "=== Installation Complete ==="
echo ""
echo "To start the daemon:"
echo "  sudo systemctl start sigil-daemon"
echo ""
echo "To check status:"
echo "  sigil status"
echo ""
echo "NOTE: You may need to log out and back in for group membership to take effect."
echo ""
