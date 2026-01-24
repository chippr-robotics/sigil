# Genesis Operations Guide

This document provides a comprehensive guide to genesis operations in the Sigil MPC signing system. Genesis operations cover the initial setup and provisioning of the mother device and agent, establishing the cryptographic foundation for all future signing operations.

## Table of Contents

1. [Overview](#overview)
2. [Security Model](#security-model)
3. [Prerequisites](#prerequisites)
4. [Mother Device Initialization](#mother-device-initialization)
5. [Agent Shard Transfer](#agent-shard-transfer)
6. [Verification Procedures](#verification-procedures)
7. [Ledger Integration](#ledger-integration)
8. [Troubleshooting](#troubleshooting)
9. [Security Best Practices](#security-best-practices)
10. [Recovery Considerations](#recovery-considerations)

---

## Overview

### What is Genesis?

Genesis is the one-time initialization process that establishes the cryptographic foundation for your Sigil MPC signing system. During genesis:

1. **Master Key Generation**: A master keypair is created using secure randomness
2. **Key Splitting**: The master private key is split into two shards using MPC principles:
   - **Cold Shard**: Remains on the air-gapped mother device
   - **Agent Shard**: Transferred to the agent device for signing operations
3. **Secure Storage**: Both shards are encrypted and stored securely on their respective devices
4. **Registry Initialization**: The mother device initializes its child registry for tracking derived keys

### Why Genesis Matters

Genesis is **the most critical operation** in the Sigil system because:

- The master key generated during genesis is the root of trust for all child keys
- The security of all future signatures depends on the security of this initial setup
- Genesis is **irreversible** - you cannot reinitialize without creating a new system
- Loss of either shard renders the system unable to sign (though a cold shard loss can be mitigated with Ledger-based recovery)

### Threat Model During Genesis

During genesis, the system is vulnerable to:

- **Weak randomness** leading to predictable keys
- **Unauthorized observation** of the agent shard during transfer
- **Man-in-the-middle attacks** if transfer is not secured
- **Hardware/firmware compromise** on either device
- **Side-channel attacks** during key generation

See [THREAT_MODEL.md](THREAT_MODEL.md) for comprehensive threat analysis.

---

## Security Model

### Two-Party Computation

Sigil uses a 2-of-2 threshold scheme where:

- **Neither shard alone** can produce valid signatures
- **Both shards are required** for all signing operations
- **Compromise of one shard** does not compromise the system (though it enables denial of service)

### Air-Gapped Architecture

```
┌─────────────────────────┐         ┌─────────────────────────┐
│   Mother Device         │         │   Agent Device          │
│   (Air-Gapped)          │         │   (Network-Connected)   │
├─────────────────────────┤         ├─────────────────────────┤
│                         │         │                         │
│  Master Key Generation  │         │   Agent Shard Import    │
│  ├─ Cold Shard (stays)  │         │   ├─ Agent Shard        │
│  └─ Agent Shard ────────┼────────►│   └─ Presig Shares      │
│                         │         │                         │
│  Child Disk Creation    │         │   Signing Operations    │
│  Presig Generation      │         │   Transaction Broadcast │
│  Reconciliation         │         │                         │
└─────────────────────────┘         └─────────────────────────┘
        ▲                                     │
        │          ┌──────────────┐          │
        └──────────│ Floppy Disk  │◄─────────┘
                   │ (Child)      │
                   └──────────────┘
```

### Shard Storage Security

| Shard | Location | Storage Method | Threat Protection |
|-------|----------|----------------|-------------------|
| Cold Shard | Mother device | Encrypted JSON file | Physical access required, air-gapped |
| Agent Shard | Agent device | Encrypted binary file, restrictive permissions (0600) | Network isolation, access controls |
| Child Presigs | Floppy disk | Bitcode-encoded, tamper-evident | Physical possession required, integrity checks |

---

## Prerequisites

### Hardware Requirements

#### Mother Device
- **Air-gapped computer** (no network connectivity)
- Minimum 4GB RAM, 20GB storage
- USB ports for floppy drive
- **Recommended**: Dedicated hardware, never connected to network
- **Optional**: Ledger Nano S/X for hardware-backed key generation

#### Agent Device
- Network-connected server or workstation
- Minimum 8GB RAM, 50GB storage
- USB ports for floppy drive
- Linux-based OS (for udev disk detection)

#### Transfer Medium
- **Option 1**: QR code scanner/generator (recommended for maximum security)
- **Option 2**: Encrypted USB drive (air-gapped transfer)
- **Option 3**: Manual transcription (32 bytes = 64 hex characters)

### Software Requirements

#### On Mother Device
```bash
# Install sigil-mother binary
cargo install --path crates/sigil-mother

# Verify installation
sigil-mother --version
```

#### On Agent Device
```bash
# Install sigil daemon and CLI
cargo install --path crates/sigil-daemon
cargo install --path crates/sigil-cli

# Verify installation
sigil-daemon --version
sigil --version
```

#### Optional: Ledger Support
```bash
# Rebuild with Ledger support
cargo build --release --features ledger -p sigil-mother

# On Linux, configure udev rules for Ledger
sudo cp scripts/ledger-udev-rules /etc/udev/rules.d/20-ledger.rules
sudo udevadm control --reload-rules
```

### Pre-Genesis Checklist

- [ ] Mother device is air-gapped (all network interfaces disabled/removed)
- [ ] Agent device is secured (firewall configured, access controls in place)
- [ ] Both devices have fresh OS installations (recommended)
- [ ] Storage directories have appropriate permissions
- [ ] Backup strategy is defined (see [RECOVERY.md](RECOVERY.md))
- [ ] Physical security measures are in place
- [ ] If using Ledger: Device initialized with 24-word seed phrase backed up

---

## Mother Device Initialization

### Standard Initialization (Software-Only)

This method generates the master key using the operating system's cryptographic random number generator.

#### Step 1: Prepare the Environment

```bash
# On mother device
cd /path/to/sigil

# Set storage directory (default: ./sigil_mother_data)
export SIGIL_MOTHER_DATA="$HOME/.sigil-mother"

# Create secure directory with restricted permissions
mkdir -p "$SIGIL_MOTHER_DATA"
chmod 700 "$SIGIL_MOTHER_DATA"
```

#### Step 2: Run Initialization

```bash
sigil-mother init
```

**Expected Output:**
```
=== Master Key Generated ===

Master Public Key: 0x02a3b4c5d6e7f8901234567890abcdef1234567890abcdef1234567890abcdef12

⚠️  IMPORTANT: The agent shard must be securely transferred to the agent device.
Agent Master Shard: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef

⚠️  Write down or securely store the agent shard, then clear your terminal.

[INFO] Master shard saved to /home/user/.sigil-mother
```

#### Step 3: Record Critical Information

**Immediately record the following in your secure backup system:**

1. **Master Public Key** (for verification)
   - Example: `0x02a3b4c5...`
   - Use: Verify child keys are derived from correct master

2. **Agent Master Shard** (for transfer to agent)
   - Example: `0x1234567890abcdef...` (64 hex characters)
   - ⚠️ **SENSITIVE**: This must be transferred securely and then deleted

3. **Cold Shard Backup** (optional but recommended)
   - Location: `$SIGIL_MOTHER_DATA/master_shard.json`
   - Backup this file to secure offline storage

#### Step 4: Verify Initialization

```bash
# Check status
sigil-mother status
```

**Expected Output:**
```
=== Mother Device Status ===

Master Public Key: 0x02a3b4c5d6e7f8901234567890abcdef1234567890abcdef1234567890abcdef12
Created: 2026-01-20 01:00:00 UTC
Next Child Index: 0

Children:
  Active:    0
  Suspended: 0
  Nullified: 0
```

#### Step 5: Secure the Terminal

```bash
# Clear terminal history to remove agent shard from view
history -c
clear

# On some systems, also clear shell history file
cat /dev/null > ~/.bash_history
```

---

### Ledger-Based Initialization (Hardware-Backed)

Using a Ledger hardware wallet provides additional security by:
- Generating keys using the Ledger's hardware TRNG (True Random Number Generator)
- Enabling deterministic recovery from Ledger's 24-word seed phrase
- Protecting against software-based random number attacks

#### Prerequisites

1. **Ledger Device Setup**
   - Device initialized with 24-word recovery phrase
   - Recovery phrase backed up securely (metal backup recommended)
   - Ethereum app installed and up to date
   - Device PIN configured

2. **System Configuration**
   - Ledger udev rules installed (Linux)
   - sigil-mother built with `--features ledger`

#### Step 1: Connect Ledger

```bash
# Connect Ledger via USB
# Unlock with PIN
# Open Ethereum app on device
```

#### Step 2: Verify Ledger Connection

```bash
sigil-mother ledger-status
```

**Expected Output (Connected):**
```
=== Ledger Device Status ===

✓ Ledger device connected
Model: Ledger Nano S Plus
✓ Ethereum app is open
Address: 0x1234567890abcdef1234567890abcdef12345678
```

**Expected Output (Not Connected):**
```
=== Ledger Device Status ===

✗ No Ledger device found

Troubleshooting:
  1. Ensure Ledger is connected via USB
  2. Unlock the device with your PIN
  3. Open the Ethereum app
  4. Check USB permissions (udev rules on Linux)
```

#### Step 3: Initialize with Ledger

```bash
sigil-mother init --ledger
```

**Device Prompts:**

You will be prompted **twice** on the Ledger device to sign derivation messages:

1. **First Prompt**: "Sigil MPC Cold Master Shard Derivation v1"
   - Review message on device
   - Press both buttons to approve

2. **Second Prompt**: "Sigil MPC Agent Master Shard Derivation v1"
   - Review message on device
   - Press both buttons to approve

**Expected Output:**
```
=== Master Key Generated (Ledger) ===

Master Public Key: 0x03fedcba0987654321fedcba0987654321fedcba0987654321fedcba09876543
Ledger Public Key: 0x041234567890abcdef...(uncompressed)

✓ RECOVERY: Both shards can be recovered from your Ledger's seed phrase.
  Keep your Ledger's 24-word recovery phrase safe - it backs up these keys.

⚠️  IMPORTANT: The agent shard must be securely transferred to the agent device.
Agent Master Shard: 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890

⚠️  Write down or securely store the agent shard, then clear your terminal.

[INFO] Master shard saved to /home/user/.sigil-mother
```

#### Step 4: Understand Ledger Recovery

With Ledger-based initialization:

- **Both shards are deterministically derived** from Ledger signatures
- **Recovery is possible** using the same Ledger (or restored from seed phrase)
- **To recover**: Initialize a new mother device with the same Ledger using `init --ledger`
- **The derivation messages must remain constant** (they are hardcoded in the software)

#### Step 5: Backup Strategy

**Ledger Seed Phrase (24 words)**
- Primary backup: Metal backup (e.g., Cryptosteel)
- Secondary backup: Paper in fireproof safe
- Tertiary backup: Split backup (Shamir's Secret Sharing, 3-of-5 recommended)
- **NEVER** store digitally or in plaintext

**Cold Shard File** (Optional redundancy)
- Encrypted backup to offline storage
- Encrypted cloud backup (with strong passphrase)
- **Note**: Redundant if Ledger recovery phrase is backed up

---

## Agent Shard Transfer

The agent shard is the agent's **portion** of the master key. It must be transferred from the mother device to the agent device securely.

### Security Requirements

- **Confidentiality**: Agent shard must not be observed by unauthorized parties
- **Integrity**: Agent shard must not be modified during transfer
- **No Persistence**: Agent shard must be deleted from transfer medium after import
- **No Network**: Transfer must not traverse network connections

### Transfer Methods

#### Method 1: QR Code (Recommended)

**Most secure** for air-gapped transfer with minimal attack surface.

**On Mother Device:**
```bash
# Generate QR code (requires qrencode)
echo "0x[agent_shard_hex]" | qrencode -t UTF8

# Or use built-in Sigil QR generation (future feature)
sigil-mother export-agent-shard --qr
```

**On Agent Device:**
```bash
# Scan QR code with camera (future feature)
sigil import-agent-shard --qr

# Or manually type the hex string
sigil import-agent-shard --hex "0x[scanned_hex_from_qr]"
```

**Advantages:**
- No physical transfer medium required
- Visual verification possible
- Minimal attack surface

**Disadvantages:**
- Requires camera/display
- Susceptible to visual observation (use in secure room)

#### Method 2: Encrypted USB Drive

**Suitable** when QR is not available, provides encryption at rest.

**On Mother Device:**
```bash
# Create encrypted container
AGENT_SHARD="0x1234567890abcdef..."

# Encrypt with GPG (using symmetric passphrase)
echo "$AGENT_SHARD" | gpg --symmetric --cipher-algo AES256 > /media/usb/agent_shard.gpg

# Verify file was created
ls -lh /media/usb/agent_shard.gpg

# Unmount USB
umount /media/usb
```

**On Agent Device:**
```bash
# Mount USB (read-only)
mount -o ro /dev/sdX /media/usb

# Decrypt and import
gpg --decrypt /media/usb/agent_shard.gpg | sigil import-agent-shard --file -

# Or decrypt to temporary file
gpg --decrypt /media/usb/agent_shard.gpg > /tmp/agent_shard.txt
sigil import-agent-shard --file /tmp/agent_shard.txt

# Securely delete temporary file
shred -vfz -n 10 /tmp/agent_shard.txt
```

**Secure USB Erasure:**
```bash
# Overwrite entire USB drive
dd if=/dev/zero of=/dev/sdX bs=1M status=progress

# Or use secure erase
hdparm --security-erase /dev/sdX
```

#### Method 3: Manual Transcription

**Most secure** against electronic attacks, but prone to human error.

**Process:**
1. Write down the 64 hex characters on paper
2. Physically transport the paper to agent device
3. Manually type into agent device
4. Verify checksum
5. Securely destroy paper (shred and burn)

**On Mother Device:**
```bash
# Display agent shard with checksum
AGENT_SHARD="0x1234567890abcdef..."
echo "Agent Shard: $AGENT_SHARD"
echo "SHA256 Checksum: $(echo -n "$AGENT_SHARD" | sha256sum)"
```

**On Agent Device:**
```bash
# Import with manual entry
sigil import-agent-shard --hex "0x[manually_typed_hex]"

# Verify checksum matches
echo -n "0x[manually_typed_hex]" | sha256sum
```

### Import Process

#### Step 1: Start Agent Daemon

```bash
# On agent device, start daemon
sudo sigil-daemon --socket /tmp/sigil.sock &

# Verify daemon is running
sigil status
```

**Expected Output:**
```
Sigil daemon v0.1.0 is running
```

#### Step 2: Import Agent Shard

**From Hex String:**
```bash
sigil import-agent-shard --hex "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
```

**From File:**
```bash
sigil import-agent-shard --file /path/to/agent_shard.txt
```

**Expected Output:**
```
✓ Agent shard imported successfully
The agent shard is now stored securely and ready for signing operations.
```

#### Step 3: Verify Import

```bash
# Check that agent shard is loaded
sigil status
```

**Expected Output:**
```
Sigil daemon v0.1.0 is running
Agent master shard: ✓ Loaded
Children imported: 0
```

#### Step 4: Secure Cleanup

```bash
# Clear terminal history
history -c
clear

# If agent shard was in a file, securely delete it
shred -vfz -n 10 /path/to/agent_shard.txt

# Verify deletion
ls -la /path/to/agent_shard.txt  # Should show "No such file or directory"
```

### Security Verification

After import, verify the agent shard is stored securely:

```bash
# Check file permissions (should be 0600)
ls -la ~/.sigil/agent_store/agent_master_shard.bin

# Expected output:
# -rw------- 1 user user 32 Jan 20 01:00 agent_master_shard.bin
```

---

## Verification Procedures

### End-to-End Verification

After completing genesis operations, perform these verification steps to ensure the system is correctly configured.

#### Test 1: Mother Device Status

```bash
# On mother device
sigil-mother status
```

**Verify:**
- ✓ Master public key is displayed (starts with 0x02 or 0x03)
- ✓ Created timestamp is recent
- ✓ Next child index is 0
- ✓ No children registered yet

#### Test 2: Agent Status

```bash
# On agent device
sigil status
```

**Verify:**
- ✓ Daemon is running
- ✓ Agent master shard is loaded
- ✓ No children imported yet

#### Test 3: Create Test Child

This verifies that both shards work together correctly.

```bash
# On mother device, create a test child
sigil-mother create-child \
  --presig-count 10 \
  --output /tmp/test_child.img \
  --agent-output /tmp/test_child_agent.json
```

**Expected Output:**
```
=== Child Created ===

Child ID: 7a3f2c1b
Public Key: 0x02abcdef1234567890abcdef1234567890abcdef1234567890abcdef12345678
Derivation Path: m/44'/60'/0'/0/0
Presigs: 10

Disk image: /tmp/test_child.img
Agent shares: /tmp/test_child_agent.json

⚠️  Securely transfer agent shares to the agent device, then delete the file.
```

**Verify:**
- ✓ Child created successfully
- ✓ Child ID generated (8 hex characters)
- ✓ Public key is valid (33 bytes compressed)
- ✓ Files created at specified paths

#### Test 4: Import and Sign

```bash
# Transfer agent shares to agent device (using secure method)

# On agent device, import child shares
sigil import-child-shares /path/to/test_child_agent.json

# Insert test child disk into agent device

# Verify disk is detected
sigil disk
```

**Expected Output:**
```
Disk detected: sigil_7a3f2c1b
Presigs: 10/10 remaining
Expires: 30 days
Valid: Yes
```

```bash
# Sign a test message
sigil sign \
  --message 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef \
  --chain-id 1 \
  --description "Genesis verification test"
```

**Expected Output:**
```
Signing message...
Signature: 0xabcdef1234...
Presig index: 0
Proof hash: 0x9876543210...
```

**Verify:**
- ✓ Signature produced successfully
- ✓ Signature is 65 bytes (130 hex characters)
- ✓ Presig index is 0 (first signature)
- ✓ Proof hash generated

#### Test 5: Signature Verification

Use an external tool to verify the signature:

```bash
# Using ethers.js (Node.js)
node -e "
const ethers = require('ethers');
const messageHash = '0x1234567890abcdef...';
const signature = '0xabcdef1234...';
const recovered = ethers.utils.recoverAddress(messageHash, signature);
console.log('Recovered address:', recovered);
"
```

**Verify:**
- ✓ Recovered address matches child public key

### Master Public Key Verification

To ensure both devices are using the same master key:

```bash
# On mother device
sigil-mother status | grep "Master Public Key"
# Output: Master Public Key: 0x02abcd...

# Derive first child address (future feature)
sigil-mother derive-address --index 0
# Output: 0x1234567890abcdef1234567890abcdef12345678
```

**Compare this with the child public key from Test 3. They should match.**

---

## Ledger Integration

### Ledger Setup

#### Initial Ledger Configuration

1. **Unbox and Initialize**
   - Connect Ledger to computer
   - Follow on-screen setup instructions
   - Set a strong PIN (8 digits recommended)

2. **Record Recovery Phrase**
   - Write down all 24 words in order
   - Use a metal backup (e.g., Cryptosteel, Billfodl)
   - Store in secure location (fireproof safe, bank vault)
   - **NEVER** take a photo or store digitally

3. **Install Ethereum App**
   - Open Ledger Live
   - Go to "Manager"
   - Install "Ethereum" app
   - Keep firmware up to date

4. **Configure Linux Permissions** (Linux only)
   ```bash
   # Create udev rules file
   sudo tee /etc/udev/rules.d/20-ledger.rules > /dev/null <<EOF
   SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", MODE="0660", GROUP="plugdev"
   EOF

   # Reload udev rules
   sudo udevadm control --reload-rules
   sudo udevadm trigger

   # Add user to plugdev group
   sudo usermod -a -G plugdev $USER

   # Log out and back in for group change to take effect
   ```

### Using Ledger for Genesis

See [Ledger-Based Initialization](#ledger-based-initialization-hardware-backed) section above for complete procedure.

### Ledger Recovery Procedure

If you need to recover a Ledger-based genesis:

1. **Obtain Ledger device** (original or restored from seed phrase)
2. **Restore from seed phrase** (if using new device)
3. **Install Ethereum app** (must be same version or compatible)
4. **Run genesis again**:
   ```bash
   sigil-mother init --ledger
   ```
5. **Same shards will be derived** (deterministic from seed phrase)
6. **Agent shard must be re-transferred** to agent device

**Important Notes:**
- Ledger derivation messages are **hardcoded** in software
- Using different software version may produce different shards
- Always test recovery in safe environment first

### Ledger Security Considerations

**Advantages:**
- Hardware TRNG for key generation
- Private keys never leave secure element
- PIN protection
- Deterministic recovery from seed phrase

**Limitations:**
- Physical device required for recovery
- Firmware/hardware trust assumptions
- Supply chain risks (buy from official sources)
- PIN brute-force protection limited (typically 3-10 attempts)

---

## Troubleshooting

### Mother Device Issues

#### Issue: "Master shard already exists"

**Cause:** Attempting to reinitialize after genesis already completed.

**Solution:**
```bash
# Check current status
sigil-mother status

# If you need to reinitialize (DESTRUCTIVE):
# 1. Backup current data
mv ~/.sigil-mother ~/.sigil-mother.backup

# 2. Reinitialize
sigil-mother init

# Note: This creates a NEW master key. Previous children will not work.
```

#### Issue: "Permission denied" when saving master shard

**Cause:** Insufficient permissions on storage directory.

**Solution:**
```bash
# Fix permissions
chmod 700 ~/.sigil-mother
sudo chown -R $USER:$USER ~/.sigil-mother
```

#### Issue: Ledger not detected

**Cause:** USB connection, udev rules, or app not open.

**Solution:**
```bash
# Check Ledger status
sigil-mother ledger-status

# Verify USB connection
lsusb | grep Ledger

# Check udev rules
ls -la /etc/udev/rules.d/20-ledger.rules

# Restart udev
sudo udevadm control --reload-rules
sudo udevadm trigger

# Ensure Ethereum app is open on device
```

### Agent Device Issues

#### Issue: "Agent shard not imported"

**Cause:** Agent shard not yet imported or import failed.

**Solution:**
```bash
# Check daemon status
sigil status

# Import agent shard
sigil import-agent-shard --hex "0x..."

# Verify import
sigil status
```

#### Issue: "Invalid hex string"

**Cause:** Agent shard format incorrect.

**Solution:**
```bash
# Agent shard must be exactly 64 hex characters (32 bytes)
# With or without 0x prefix

# Correct format:
# 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
# or
# 1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef

# Verify length
SHARD="0x1234..."
echo ${#SHARD}  # Should be 66 (with 0x) or 64 (without)
```

#### Issue: Daemon not running

**Cause:** Daemon process not started or crashed.

**Solution:**
```bash
# Start daemon
sudo sigil-daemon --socket /tmp/sigil.sock &

# Check if running
ps aux | grep sigil-daemon

# Check logs
journalctl -u sigil-daemon -f

# Or check socket exists
ls -la /tmp/sigil.sock
```

### Network Issues

#### Issue: Cannot connect to daemon from CLI

**Cause:** Socket path mismatch or permissions.

**Solution:**
```bash
# Verify socket path
ls -la /tmp/sigil.sock

# Use matching socket path in CLI
sigil --socket /tmp/sigil.sock status

# Check socket permissions
# Should be accessible by user running CLI
```

---

## Security Best Practices

### During Genesis

1. **Environment Isolation**
   - Perform genesis in a physically secure room
   - No cameras, no observers
   - Mother device never connected to network
   - Agent device on isolated network segment

2. **Device Security**
   - Fresh OS installations recommended
   - Full disk encryption enabled
   - Strong passphrases/PINs
   - Secure boot configured
   - Regular security updates (after genesis)

3. **Transfer Security**
   - Use QR codes when possible
   - Encrypt USB transfers
   - Manual transcription in secure environment
   - Multiple verification steps
   - Immediate cleanup after transfer

4. **Documentation**
   - Record master public key securely
   - Document backup locations
   - Create recovery procedures
   - Store in multiple locations
   - Regular backup verification

### Post-Genesis

1. **Access Controls**
   - Restrict physical access to mother device
   - Multi-factor authentication on agent device
   - Role-based access controls
   - Audit logging enabled
   - Regular access reviews

2. **Operational Security**
   - Regular security audits
   - Incident response procedures
   - Disaster recovery testing
   - Staff training
   - Secure disposal procedures

3. **Monitoring**
   - Child creation monitoring
   - Signing activity monitoring
   - Anomaly detection
   - Alert thresholds
   - Regular reconciliation

### What NOT to Do

❌ **Never store agent shard in plaintext on network-accessible storage**
❌ **Never initialize mother device while connected to network**
❌ **Never photograph or screenshot agent shard**
❌ **Never email or transmit agent shard over network**
❌ **Never skip verification procedures**
❌ **Never reuse master key across different systems**
❌ **Never store Ledger seed phrase digitally**

---

## Recovery Considerations

### Recovery Strategies

Genesis operations should be performed with recovery in mind:

1. **Ledger-Based Recovery**
   - Pros: Deterministic, seed phrase is standard
   - Cons: Requires compatible Ledger, firmware dependency
   - Best for: Long-term recovery capability

2. **Cold Shard Backup**
   - Pros: Software-independent, can be encrypted
   - Cons: Single point of failure, storage requirements
   - Best for: Redundancy with Ledger recovery

3. **Shamir's Secret Sharing**
   - Pros: Distributed trust, no single point of failure
   - Cons: Complexity, requires threshold of shares
   - Best for: High-security environments

### Backup Checklist

After genesis, ensure you have:

- [ ] Master public key recorded (for verification)
- [ ] Cold shard backed up (encrypted, offline storage)
- [ ] Agent shard backup (encrypted, separate from cold shard)
- [ ] Ledger seed phrase backed up (if using Ledger)
- [ ] Recovery procedures documented
- [ ] Backup locations documented (not with backups)
- [ ] Recovery tested in safe environment
- [ ] Regular backup verification scheduled

### Testing Recovery

**Test recovery in a safe environment before production use:**

```bash
# Create test genesis
sigil-mother --data-dir /tmp/test-genesis init

# Backup
cp -r /tmp/test-genesis /tmp/test-genesis-backup

# Simulate disaster
rm -rf /tmp/test-genesis

# Attempt recovery
# For Ledger: sigil-mother --data-dir /tmp/test-genesis init --ledger
# For backup: cp -r /tmp/test-genesis-backup /tmp/test-genesis

# Verify recovery
sigil-mother --data-dir /tmp/test-genesis status
```

For complete recovery procedures, see [RECOVERY.md](RECOVERY.md).

---

## Related Documentation

- **[THREAT_MODEL.md](THREAT_MODEL.md)**: Comprehensive threat analysis
- **[RECOVERY.md](RECOVERY.md)**: Disaster recovery procedures
- **[E2E_TEST_PLAN.md](E2E_TEST_PLAN.md)**: Testing procedures including genesis
- **[CRYPTO_SPEC.md](CRYPTO_SPEC.md)**: Cryptographic specifications
- **[SECURITY.md](../SECURITY.md)**: Security policy and reporting

---

## Glossary

- **Genesis**: Initial system setup, master key generation
- **Master Key**: Root keypair for all derived keys
- **Cold Shard**: Mother device's portion of master private key
- **Agent Shard**: Agent device's portion of master private key
- **Mother Device**: Air-gapped device for key generation and child creation
- **Agent Device**: Network-connected device for signing operations
- **Child**: Derived keypair with presignatures for signing
- **Presignature**: Pre-computed signature component for ECDSA signing
- **MPC**: Multi-Party Computation, cryptographic protocol for distributed computation

---

**Document Version:** 1.0  
**Last Updated:** 2026-01-20  
**Sigil Version:** v0.1.0+
