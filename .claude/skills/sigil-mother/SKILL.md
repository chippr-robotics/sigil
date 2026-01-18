# Sigil Mother Device Management

Air-gapped mother device for managing MPC signing key material. The mother device holds the cold master shard and manages the lifecycle of child signing disks.

## Security Model

The mother device contains critical key material and should be:
- Stored on an **encrypted USB drive** (LUKS full-disk encryption)
- Kept physically secure and air-gapped when not in use
- Only connected when performing mother operations (create, reconcile, refill, nullify)

## Prerequisites

- `sigil-mother` binary built and available
- Empty USB drive for mother storage (16GB+ recommended)
- Floppy disk(s) for child signing disks

## Quick Reference

| Operation | Command |
|-----------|---------|
| Initialize encrypted USB | `sigil-mother init --encrypted` |
| Create child disk | `sigil-mother create-child` |
| Check status | `sigil-mother status` |
| Reconcile child | `sigil-mother reconcile --disk <path>` |
| Refill child | `sigil-mother refill --disk <path>` |
| Nullify child | `sigil-mother nullify --child-id <id>` |
| List children | `sigil-mother list-children` |

---

## Initial Setup: Encrypted USB Storage

### Step 1: Identify USB Drive

```bash
# List block devices to find your USB drive
lsblk -o NAME,SIZE,TYPE,MOUNTPOINT,LABEL

# Example output - identify your USB (e.g., /dev/sdX)
# sdg  14.9G disk              <- Your USB drive
```

### Step 2: Create Encrypted Partition (LUKS)

**WARNING: This will erase all data on the USB drive!**

```bash
# Create GPT partition table and single partition
sudo parted /dev/sdX --script mklabel gpt mkpart primary 0% 100%

# Format with LUKS encryption (you'll be prompted for passphrase)
sudo cryptsetup luksFormat /dev/sdX1

# Open the encrypted volume
sudo cryptsetup open /dev/sdX1 sigil_mother

# Create filesystem with label
sudo mkfs.ext4 -L SIGIL_MOTHER /dev/mapper/sigil_mother

# Mount the encrypted volume
sudo mkdir -p /media/SIGIL_MOTHER
sudo mount /dev/mapper/sigil_mother /media/SIGIL_MOTHER
sudo chown $USER:$USER /media/SIGIL_MOTHER
```

### Step 3: Initialize Mother Device

```bash
sigil-mother --data-dir /media/SIGIL_MOTHER init
```

**Output:**
```
=== Sigil Mother Device Initialized ===

Master Public Key: 0x036a621c...
Agent Master Shard: 0x8d10155a... (SAVE THIS SECURELY!)

CRITICAL: Record the Agent Master Shard above.
    It is required for daemon setup and CANNOT be recovered.
```

### Step 4: Secure Unmount

```bash
# Sync and unmount
sync
sudo umount /media/SIGIL_MOTHER

# Close encrypted volume
sudo cryptsetup close sigil_mother

# Remove USB and store securely
```

---

## Mounting Encrypted Mother USB

When you need to perform mother operations:

```bash
# Open encrypted volume (enter passphrase when prompted)
sudo cryptsetup open /dev/sdX1 sigil_mother

# Mount
sudo mount /dev/mapper/sigil_mother /media/SIGIL_MOTHER

# Perform operations...

# When done - secure unmount
sync
sudo umount /media/SIGIL_MOTHER
sudo cryptsetup close sigil_mother
```

---

## Creating a New Child Disk

Creates a new signing floppy with presignatures.

### Prerequisites
- Mother USB mounted at `/media/SIGIL_MOTHER`
- Blank floppy disk inserted
- Agent master shard (from initial setup)

### Command

```bash
sigil-mother --data-dir /media/SIGIL_MOTHER create-child \
  --presig-count 1000 \
  --output /media/dontpanic/FLOPPY/sigil.disk \
  --agent-output /tmp/agent_shares.json \
  --agent-shard "0x8d10155a..."  # Your agent master shard
```

**Output:**
```
=== Child Disk Created ===

Child ID: 7a3f8b2c
Derivation: m/44'/60'/0'/0'
Presigs: 1000
Expires: 30 days

Agent shares written to: /tmp/agent_shares.json
Import agent shares to daemon, then DELETE this file!
```

### Import Agent Shares to Daemon

```bash
sigil-cli import-shares --file /tmp/agent_shares.json

# Securely delete after import
shred -u /tmp/agent_shares.json
```

---

## Checking Mother Status

```bash
sigil-mother --data-dir /media/SIGIL_MOTHER status
```

**Output:**
```
=== Mother Device Status ===

Master Public Key: 0x036a621c2b79d54e...
Created: 2026-01-18 00:50:07 UTC
Next Child Index: 5

Children:
  Active:    4
  Suspended: 0
  Nullified: 1
```

---

## Listing All Children

```bash
sigil-mother --data-dir /media/SIGIL_MOTHER list-children
```

**Output:**
```
Child ID    Status     Derivation        Created              Signatures
────────────────────────────────────────────────────────────────────────
048933a4    Active     m/44'/60'/0'/3'   2026-01-18 01:24     95
7f77e686    Active     m/44'/60'/0'/2'   2026-01-18 00:58     0
662815ee    Active     m/44'/60'/0'/0'   2026-01-17 23:50     0
29f77470    Nullified  m/44'/60'/0'/1'   2026-01-18 00:56     0
```

---

## Reconciliation

Reconciliation syncs the child disk's usage with the mother, verifying all signatures were properly logged.

### When to Reconcile
- Before refilling a disk with new presignatures
- Periodically to maintain audit trail
- When transferring disk custody

### Command

```bash
sigil-mother --data-dir /media/SIGIL_MOTHER reconcile \
  --disk /media/dontpanic/SIGIL001/sigil.disk
```

**Output:**
```
=== Reconciliation Report ===

Child ID: 048933a4
Presig Status:
  Total:  100
  Used:   95
  Fresh:  5
  Voided: 0

Log Entries: 95
All entries verified

Recommendation: RefillApproved
```

### Reconciliation Statuses

| Status | Meaning |
|--------|---------|
| `RefillApproved` | Safe to refill with new presigs |
| `AuditRequired` | Discrepancies found, manual review needed |
| `Suspended` | Disk suspended pending investigation |
| `NullifyRecommended` | Serious issues, consider nullifying |

---

## Refilling a Child Disk

Adds new presignatures to an existing child disk after reconciliation.

### Prerequisites
- Successful reconciliation (RefillApproved status)
- Mother USB and child floppy both connected
- Agent master shard

### Command

```bash
sigil-mother --data-dir /media/SIGIL_MOTHER refill \
  --disk /media/dontpanic/SIGIL001/sigil.disk \
  --presig-count 100 \
  --agent-output /tmp/refill_agent_shares.json
```

**Output:**
```
=== Disk Refilled ===

Child ID: 048933a4
New Presigs: 100
Total Available: 105 (5 existing + 100 new)
Refill Count: 1

Agent shares written to: /tmp/refill_agent_shares.json
Import to daemon, then DELETE!
```

### Import New Agent Shares

```bash
# Append new shares to daemon's agent store
sigil-cli import-shares --file /tmp/refill_agent_shares.json --append

# Securely delete
shred -u /tmp/refill_agent_shares.json
```

---

## Nullifying a Child

Permanently disables a child key. Use when:
- Disk is lost or stolen
- Disk is damaged beyond recovery
- Key compromise suspected
- Retiring an address

### WARNING: Nullification is PERMANENT and IRREVERSIBLE!

### Command

```bash
sigil-mother --data-dir /media/SIGIL_MOTHER nullify \
  --child-id 048933a4 \
  --reason "Disk lost during travel"
```

**Output:**
```
=== Child Nullification ===

WARNING: This action is PERMANENT and IRREVERSIBLE!

Child ID: 048933a4
Address: 0x32D5F595d167ABd08b09dC177FBbc481Ea5802f2
Derivation: m/44'/60'/0'/3'
Signatures to date: 95

Reason: Disk lost during travel

Type 'NULLIFY 048933a4' to confirm: NULLIFY 048933a4

Child nullified successfully.
  - Nullifier recorded in registry
  - No further signatures possible
  - Address should no longer receive funds
```

### After Nullification
1. Remove the child from daemon's agent store
2. Transfer any remaining funds from the address
3. Update any systems using this address

---

## Complete Workflow Example

### Setting Up a New Signing System

```bash
# 1. Prepare encrypted USB (one-time)
sudo cryptsetup luksFormat /dev/sdg1
sudo cryptsetup open /dev/sdg1 sigil_mother
sudo mkfs.ext4 -L SIGIL_MOTHER /dev/mapper/sigil_mother
sudo mount /dev/mapper/sigil_mother /media/SIGIL_MOTHER
sudo chown $USER:$USER /media/SIGIL_MOTHER

# 2. Initialize mother device
sigil-mother --data-dir /media/SIGIL_MOTHER init
# SAVE THE AGENT MASTER SHARD!

# 3. Create first child disk (insert floppy)
sigil-mother --data-dir /media/SIGIL_MOTHER create-child \
  --presig-count 1000 \
  --output /media/dontpanic/SIGIL001/sigil.disk \
  --agent-output /tmp/agent_shares.json \
  --agent-shard "0x..."

# 4. Set up daemon with agent shares
sigil-cli import-shares --file /tmp/agent_shares.json
shred -u /tmp/agent_shares.json

# 5. Secure mother USB
sync && sudo umount /media/SIGIL_MOTHER
sudo cryptsetup close sigil_mother
# Store USB securely!

# 6. Start daemon (child floppy inserted)
sigil-daemon &
```

### Regular Refill Cycle

```bash
# 1. Mount mother USB
sudo cryptsetup open /dev/sdg1 sigil_mother
sudo mount /dev/mapper/sigil_mother /media/SIGIL_MOTHER

# 2. Reconcile (with child floppy inserted)
sigil-mother --data-dir /media/SIGIL_MOTHER reconcile \
  --disk /media/dontpanic/SIGIL001/sigil.disk

# 3. Refill if approved
sigil-mother --data-dir /media/SIGIL_MOTHER refill \
  --disk /media/dontpanic/SIGIL001/sigil.disk \
  --presig-count 1000 \
  --agent-output /tmp/refill_shares.json

# 4. Import new shares to daemon
sigil-cli import-shares --file /tmp/refill_shares.json --append
shred -u /tmp/refill_shares.json

# 5. Secure mother USB
sync && sudo umount /media/SIGIL_MOTHER
sudo cryptsetup close sigil_mother
```

---

## Backup Recommendations

### Mother USB
- Create encrypted backup of mother data
- Store backup in separate physical location
- Test restore procedure periodically

### Agent Master Shard
- Record on paper/metal (offline)
- Store in safe deposit box or fireproof safe
- NEVER store digitally outside encrypted mother USB

### Child Registry
- Exported during backup of mother USB
- Contains all child derivation paths
- Critical for recovery scenarios

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "Device not found" | Ensure USB is connected and partition exists |
| "Wrong passphrase" | LUKS passphrase is case-sensitive |
| "Reconciliation failed" | Check disk is inserted and readable |
| "Refill denied" | Must reconcile first with approved status |
| "Cannot nullify" | Child may already be nullified |

---

## Security Checklist

- [ ] Mother USB uses LUKS encryption
- [ ] Strong passphrase (20+ characters)
- [ ] Agent master shard recorded offline
- [ ] Mother USB stored in secure location
- [ ] Temporary files shredded after use
- [ ] Regular reconciliation performed
- [ ] Backup of mother data exists
