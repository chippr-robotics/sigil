# Sigil End-to-End Test Plan

This document provides a comprehensive manual testing guide for the Sigil MPC signing system. Testers can walk through these procedures using MCP and any compatible agent (Claude Desktop, VS Code, or custom agents).

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Test Environment Setup](#test-environment-setup)
3. [Test Suite 1: Genesis Operations](#test-suite-1-genesis-operations)
4. [Test Suite 2: Child Disk Creation](#test-suite-2-child-disk-creation)
5. [Test Suite 3: Transaction Signing (ECDSA)](#test-suite-3-transaction-signing-ecdsa)
6. [Test Suite 4: FROST Operations](#test-suite-4-frost-operations)
7. [Test Suite 5: Signature Exhaustion](#test-suite-5-signature-exhaustion)
8. [Test Suite 6: Reconciliation](#test-suite-6-reconciliation)
9. [Test Suite 7: Nullification](#test-suite-7-nullification)
10. [Test Suite 8: Error Handling & Edge Cases](#test-suite-8-error-handling--edge-cases)
11. [Test Result Recording](#test-result-recording)

---

## Prerequisites

### Hardware Requirements
- [ ] Mother device (air-gapped computer)
- [ ] Agent device (computer with network access)
- [ ] USB floppy drive (or USB drive formatted as floppy)
- [ ] Blank 1.44MB floppy disks (minimum 3 for testing)
- [ ] Optional: Ledger hardware wallet (Nano S/X)

### Software Requirements
- [ ] Sigil binaries compiled and installed:
  - `sigil-mother` (on mother device)
  - `sigil-daemon` (on agent device)
  - `sigil-mcp` (on agent device)
  - `sigil` (on agent device)
- [ ] MCP-compatible agent installed:
  - Claude Desktop, or
  - VS Code with Claude extension, or
  - Custom MCP client
- [ ] MCP server configuration added to agent

### MCP Configuration

Add to your agent's MCP configuration (e.g., `~/.config/claude-desktop/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "sigil": {
      "command": "sigil-mcp",
      "args": ["--daemon-socket", "/var/run/sigil/daemon.sock"]
    }
  }
}
```

### Test Data Preparation

Prepare test message hashes for signing:

```
# EVM test message hash (Ethereum transaction)
TEST_EVM_HASH_1=0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
TEST_EVM_HASH_2=0xfedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321

# Bitcoin Taproot test message hash
TEST_BTC_HASH_1=0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890

# Solana test message hash
TEST_SOL_HASH_1=0x9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba
```

---

## Test Environment Setup

### Setup Step 1: Start Sigil Daemon

On the agent device:

```bash
# Start the daemon in the background
sudo sigil-daemon --socket /var/run/sigil/daemon.sock &

# Verify daemon is running
sudo sigil status
```

**Expected Result:**
- Daemon starts without errors
- Status shows "Daemon running, no disk detected"

### Setup Step 2: Verify MCP Server

On the agent device:

```bash
# start the agents MCP server
sigil-mcp &
```

```bash
# Test MCP server manually with curl
curl -X POST http://localhost:3000/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}'

**Expected Result:**
- JSON response with server capabilities
- Tools listed: `sigil_check_disk`, `sigil_sign_evm`, `sigil_sign_frost`, etc.

### Setup Step 3: Verify Agent Connection

In your MCP agent, type:
> "Check if Sigil is available"

**Expected Result:**
- Agent confirms Sigil MCP server is connected
- Lists available Sigil tools

---

## Test Suite 1: Genesis Operations

### Test 1.1: Initialize Mother Device (Standard)

**Objective:** Create master key shards without hardware wallet

**Location:** Air-gapped mother device

**Procedure:**

1. On the mother device, run:
   ```bash
   sigil-mother init
   ```

2. Follow the on-screen prompts:
   - Confirm you understand this is a one-time operation
   - Accept storage location for encrypted master shard

3. Record the displayed information:
   - Master public key (hex)
   - Backup recovery phrase (if applicable)

**Expected Results:**
- [ ] Master cold shard generated and stored encrypted
- [ ] Master agent shard file created (`master_shard.json`)
- [ ] Master public key displayed
- [ ] No errors during generation

**Pass Criteria:**
- Both shards successfully created
- Public key can be derived from combined shards

---

### Test 1.2: Initialize Mother Device (Ledger Hardware Wallet)

**Objective:** Create master key shards using Ledger for cold shard derivation

**Location:** Air-gapped mother device with Ledger connected

**Prerequisites:**
- Ledger device initialized with recovery phrase
- Ethereum app installed and open on Ledger

**Procedure:**

1. Connect Ledger to mother device

2. Run:
   ```bash
   sigil-mother init --ledger
   ```

3. On Ledger device:
   - Review derivation message
   - Approve signature

4. Record displayed information

**Expected Results:**
- [ ] Ledger detected and connected
- [ ] Derivation message signed on device
- [ ] Cold shard derived deterministically from signature
- [ ] Agent shard generated with OsRng
- [ ] Same recovery possible with same Ledger + same message

**Pass Criteria:**
- Master shards created successfully
- Ledger involvement confirmed in output

---

### Test 1.3: Transfer Agent Shard to Agent Device

**Objective:** Securely transfer the agent's portion of the master key to the agent device

**Background:** During initialization, the master key is split into two shards:
- **Cold shard**: Stays on the air-gapped mother device
- **Agent shard**: Must be transferred to the agent device for signing operations

**Procedure:**

1. On mother device, the agent shard is displayed during `sigil-mother init`:
   ```
   Agent Master Shard: 0x[64 hex characters]
   ```
   Record this securely (never store plaintext on disk if possible)

2. Transfer securely to agent device using one of these methods:
   - QR code scan (recommended for air-gapped security)
   - Encrypted USB drive (decrypt on agent device)
   - Manual transcription for ultimate security (32 bytes = 64 hex chars)

3. On agent device, import the agent shard:
   ```bash
   # Option 1: From hex string
   sigil import-agent-shard --hex "0x[agent_shard_hex]"
   
   # Option 2: From file (if transferred via encrypted USB)
   sigil import-agent-shard --file agent_shard.txt
   ```

4. Securely delete any transfer media and clear terminal history:
   ```bash
   # Clear terminal history
   history -c
   
   # Overwrite and delete any transfer files
   shred -vfz -n 10 agent_shard.txt
   ```

**Expected Results:**
- [ ] Agent shard imported to agent store (~/.sigil/agent_store/)
- [ ] Agent shard file encrypted at rest on agent device
- [ ] Shard securely deleted from transfer medium
- [ ] Agent can verify shard loaded with `sigil status`

**Pass Criteria:**
- Agent can verify shard is loaded and ready
- No plaintext copies of shard remain on transfer medium or in terminal history
- Agent store shows "Agent master shard: ✓ Loaded"

**Security Notes:**
- The agent shard is the agent's **portion** of the master key, not the complete master key
- Both the cold shard (on mother) and agent shard are required for signing
- The agent shard should be treated as highly sensitive cryptographic material

---

## Test Suite 2: Child Disk Creation

### Test 2.1: Create First Child Disk

**Objective:** Generate a child disk with default presignature count

**Location:** Mother device

**Procedure:**

1. Insert blank floppy disk into mother device

2. Run:
   ```bash
   sigil-mother create-child \
     --presig-count 1000 \
     --agent-output child1_agent_shares.json
   ```

3. Record displayed information:
   - Child ID (short format)
   - Child public key
   - Derivation path
   - Expiration date

4. Verify disk was written:
   ```bash
   sigil-mother verify-disk /dev/fd0
   ```

**Expected Results:**
- [ ] Child ID generated (SHA256 of public key)
- [ ] 1000 presignatures generated
- [ ] Disk image written to floppy
- [ ] Agent shares file created
- [ ] Mother signature present on disk header

**Verification Checklist:**
- [ ] `sigil-mother verify-disk` passes all checks
- [ ] Disk size approximately 1.44MB
- [ ] Header magic bytes correct ("SIGILDSK")

---

### Test 2.2: Import Child Agent Shares

**Objective:** Load agent shares for the created child

**Location:** Agent device

**Procedure:**

1. Transfer `child1_agent_shares.json` to agent device

2. Import shares:
   ```bash
   sigil import-child-shares child1_agent_shares.json
   ```

3. Verify import:
   ```bash
   sigil list-children
   ```

**Expected Results:**
- [ ] Shares imported successfully
- [ ] Child appears in list with correct presig count
- [ ] Status shows "Ready" with 1000 presigs available

---

### Test 2.3: Create Child with Reduced Presignatures (Testing)

**Objective:** Create a child disk with fewer presigs for exhaustion testing

**Procedure:**

1. Run on mother device:
   ```bash
   sigil-mother create-child \
     --presig-count 10 \
     --agent-output child2_agent_shares.json
   ```

2. Record child ID for Test Suite 5 (Exhaustion)

**Expected Results:**
- [ ] Child created with only 10 presignatures
- [ ] Faster creation time compared to 1000 presigs

---

### Test 2.4: Create FROST-Enabled Child (Taproot)

**Objective:** Create a child disk for Bitcoin Taproot signing

**Procedure:**

1. Run on mother device:
   ```bash
   sigil-mother create-child \
     --scheme taproot \
     --presig-count 100 \
     --agent-output child_taproot_shares.json
   ```

**Expected Results:**
- [ ] Taproot key pair generated
- [ ] FROST nonces generated (not ECDSA presigs)
- [ ] Child marked with scheme "taproot"

---

### Test 2.5: Create FROST-Enabled Child (Ed25519)

**Objective:** Create a child disk for Solana/Cosmos signing

**Procedure:**

1. Run on mother device:
   ```bash
   sigil-mother create-child \
     --scheme ed25519 \
     --presig-count 100 \
     --agent-output child_ed25519_shares.json
   ```

**Expected Results:**
- [ ] Ed25519 key pair generated
- [ ] FROST nonces for Ed25519 scheme
- [ ] Child marked with scheme "ed25519"

---

## Test Suite 3: Transaction Signing (ECDSA)

### Test 3.1: Check Disk Status via MCP

**Objective:** Verify disk detection and status reporting

**Procedure:**

1. Insert child disk into agent device

2. In your MCP agent, type:
   > "Check the Sigil disk status"

3. Or call the tool directly:
   ```json
   {
     "tool": "sigil_check_disk",
     "arguments": {}
   }
   ```

**Expected Results:**
- [ ] Disk detected
- [ ] Child ID matches created disk
- [ ] Presig count shows 1000 (or created amount)
- [ ] Days until expiry shown (should be ~30)
- [ ] `is_valid` returns true

**Sample Expected Output:**
```json
{
  "detected": true,
  "child_id": "7a3f2c1b",
  "scheme": "ecdsa",
  "presigs_remaining": 1000,
  "presigs_total": 1000,
  "days_until_expiry": 30,
  "is_valid": true
}
```

---

### Test 3.2: Get Ethereum Address

**Objective:** Retrieve the signing address for the disk

**Procedure:**

1. In your MCP agent, type:
   > "What's the Ethereum address for this Sigil disk?"

2. Or call directly:
   ```json
   {
     "tool": "sigil_get_address",
     "arguments": {
       "scheme": "ecdsa",
       "format": "evm"
     }
   }
   ```

**Expected Results:**
- [ ] Valid Ethereum address returned (0x...)
- [ ] Address is checksummed (EIP-55)
- [ ] Same address returned on repeated calls

---

### Test 3.3: Sign First EVM Transaction

**Objective:** Produce a valid ECDSA signature

**Procedure:**

1. In your MCP agent, type:
   > "Sign this Ethereum transaction hash: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef for chain ID 1, description: Test transfer to Alice"

2. Or call directly:
   ```json
   {
     "tool": "sigil_sign_evm",
     "arguments": {
       "message_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
       "chain_id": 1,
       "description": "Test transfer to Alice"
     }
   }
   ```

**Expected Results:**
- [ ] Signature returned (65 bytes with v, r, s)
- [ ] v is 27 or 28
- [ ] presig_index is 0 (first signature)
- [ ] proof_hash generated
- [ ] Presig count decreased by 1

**Verification:**
1. Verify signature using ethers.js or web3.py:
   ```javascript
   const { ethers } = require('ethers');
   const recovered = ethers.utils.recoverAddress(messageHash, { v, r, s });
   console.log("Recovered address:", recovered);
   // Should match the disk's Ethereum address
   ```

---

### Test 3.4: Sign Multiple Transactions Sequentially

**Objective:** Verify sequential signing and presig consumption

**Procedure:**

1. Sign 5 different message hashes sequentially

2. After each signature, verify:
   - presig_index increments
   - presig_remaining decreases
   - Each signature is unique

**Test Messages:**
```
Hash 1: 0x1111111111111111111111111111111111111111111111111111111111111111
Hash 2: 0x2222222222222222222222222222222222222222222222222222222222222222
Hash 3: 0x3333333333333333333333333333333333333333333333333333333333333333
Hash 4: 0x4444444444444444444444444444444444444444444444444444444444444444
Hash 5: 0x5555555555555555555555555555555555555555555555555555555555555555
```

**Expected Results:**
- [ ] All 5 signatures produced
- [ ] presig_index values: 0, 1, 2, 3, 4
- [ ] presigs_remaining decreased from 1000 to 995
- [ ] All signatures verify against same public key
- [ ] No duplicate signatures

---

### Test 3.5: Update Transaction Hash After Broadcast

**Objective:** Record the actual on-chain transaction hash

**Procedure:**

1. After "broadcasting" a signed transaction, record the tx hash:
   ```json
   {
     "tool": "sigil_update_tx_hash",
     "arguments": {
       "presig_index": 0,
       "tx_hash": "0xabc123def456789abc123def456789abc123def456789abc123def456789abc1"
     }
   }
   ```

**Expected Results:**
- [ ] Update accepted
- [ ] Usage log entry updated with tx_hash
- [ ] No error on valid presig_index

---

### Test 3.6: Sign for Different EVM Chains

**Objective:** Verify chain_id handling

**Procedure:**

Sign transactions for different chains:

| Chain | chain_id | Description |
|-------|----------|-------------|
| Ethereum Mainnet | 1 | Test mainnet signing |
| Polygon | 137 | Test L2 signing |
| Arbitrum | 42161 | Test L2 signing |
| Sepolia (testnet) | 11155111 | Test testnet signing |

**Expected Results:**
- [ ] All chain_ids accepted
- [ ] Signatures valid for each chain
- [ ] chain_id recorded in usage log

---

## Test Suite 4: FROST Operations

### Test 4.1: FROST DKG Ceremony (Taproot)

**Objective:** Execute Distributed Key Generation for FROST Taproot

**Location:** Both mother and agent devices (simulated or actual)

**Procedure:**

1. On mother device, initiate DKG:
   ```bash
   sigil-mother frost-dkg init \
     --scheme taproot \
     --threshold 2 \
     --parties 2 \
     --party-id 1
   ```

2. Exchange Round 1 packages:
   - Mother exports round1_package.json
   - Agent imports and exports its round1

3. Complete Round 2:
   ```bash
   sigil-mother frost-dkg round2 \
     --import agent_round1.json
   ```

4. Finalize and verify:
   ```bash
   sigil-mother frost-dkg finalize
   ```

**Expected Results:**
- [ ] Both parties complete DKG
- [ ] Verification hashes match
- [ ] Shared public key (verifying key) identical on both sides
- [ ] Key shares securely stored

---

### Test 4.2: Create FROST Child Disk (Post-DKG)

**Objective:** Create child disk from FROST key shares

**Procedure:**

1. After successful DKG, create child:
   ```bash
   sigil-mother create-frost-child \
     --scheme taproot \
     --presig-count 100 \
     --agent-output frost_child_shares.json
   ```

**Expected Results:**
- [ ] FROST nonces generated
- [ ] Child disk created with taproot scheme
- [ ] Agent shares exported for transfer

---

### Test 4.3: Get Bitcoin Taproot Address

**Objective:** Retrieve Bitcoin address from FROST key

**Procedure:**

1. Insert FROST (taproot) disk

2. In MCP agent:
   > "Get the Bitcoin address for this Sigil disk"

3. Or call:
   ```json
   {
     "tool": "sigil_get_address",
     "arguments": {
       "scheme": "taproot",
       "format": "bitcoin"
     }
   }
   ```

**Expected Results:**
- [ ] Address starts with "bc1p" (Bech32m)
- [ ] Valid P2TR address format
- [ ] Consistent across calls

---

### Test 4.4: Sign Bitcoin Taproot Transaction

**Objective:** Produce valid BIP-340 Schnorr signature

**Procedure:**

1. In MCP agent:
   > "Sign this Bitcoin transaction using FROST Taproot: [message_hash]"

2. Or call:
   ```json
   {
     "tool": "sigil_sign_frost",
     "arguments": {
       "scheme": "taproot",
       "message_hash": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
       "description": "Bitcoin test transaction"
     }
   }
   ```

**Expected Results:**
- [ ] 64-byte Schnorr signature returned
- [ ] Signature verifies with BIP-340 verification
- [ ] presig_index returned
- [ ] FROST nonce consumed

**Verification:**
```python
# Verify using python-bitcoinlib or similar
from bitcoin.core.key import verify_schnorr
result = verify_schnorr(public_key, signature, message_hash)
assert result == True
```

---

### Test 4.5: Sign Solana Transaction (Ed25519)

**Objective:** Produce valid Ed25519 signature for Solana

**Prerequisites:** Ed25519 child disk created and imported

**Procedure:**

1. Insert Ed25519 disk

2. Get Solana address:
   ```json
   {
     "tool": "sigil_get_address",
     "arguments": {
       "scheme": "ed25519",
       "format": "solana"
     }
   }
   ```

3. Sign transaction:
   ```json
   {
     "tool": "sigil_sign_frost",
     "arguments": {
       "scheme": "ed25519",
       "message_hash": "0x9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba",
       "description": "Solana SOL transfer"
     }
   }
   ```

**Expected Results:**
- [ ] Solana address in Base58 format
- [ ] 64-byte Ed25519 signature
- [ ] Signature verifies against Ed25519 verification

---

### Test 4.6: Sign Cosmos Transaction (Ed25519)

**Objective:** Produce valid Ed25519 signature for Cosmos chains

**Procedure:**

1. Get Cosmos address with custom prefix:
   ```json
   {
     "tool": "sigil_get_address",
     "arguments": {
       "scheme": "ed25519",
       "format": "cosmos",
       "cosmos_prefix": "cosmos"
     }
   }
   ```

2. Also test Osmosis prefix:
   ```json
   {
     "tool": "sigil_get_address",
     "arguments": {
       "scheme": "ed25519",
       "format": "cosmos",
       "cosmos_prefix": "osmo"
     }
   }
   ```

3. Sign transaction

**Expected Results:**
- [ ] cosmos1... address returned for Cosmos
- [ ] osmo1... address returned for Osmosis
- [ ] Same underlying key, different address encoding

---

### Test 4.7: Sign Zcash Shielded Transaction (Ristretto255)

**Objective:** Produce signature for Zcash shielded operations

**Prerequisites:** Ristretto255 child disk created

**Procedure:**

1. Insert Ristretto255 disk

2. Sign shielded transaction:
   ```json
   {
     "tool": "sigil_sign_frost",
     "arguments": {
       "scheme": "ristretto255",
       "message_hash": "0x...",
       "description": "Zcash shielded transaction"
     }
   }
   ```

**Expected Results:**
- [ ] 64-byte Ristretto255 signature
- [ ] Signature verifies with Ristretto verification

---

### Test 4.8: List Supported Schemes

**Objective:** Verify all FROST schemes are available

**Procedure:**

```json
{
  "tool": "sigil_list_schemes",
  "arguments": {}
}
```

**Expected Results:**
- [ ] Lists: ecdsa, taproot, ed25519, ristretto255
- [ ] Each scheme shows supported chains

---

## Test Suite 5: Signature Exhaustion

### Test 5.1: Exhaust All Presignatures

**Objective:** Verify behavior when presignatures run out

**Prerequisites:** Child disk with 10 presignatures (from Test 2.3)

**Procedure:**

1. Insert the 10-presig disk
2. Sign 10 transactions sequentially
3. Attempt to sign an 11th transaction

**Expected Results:**
- [ ] First 10 signatures succeed
- [ ] 11th signature fails with clear error
- [ ] Error indicates presig exhaustion
- [ ] Disk status shows 0 remaining

**Expected Error:**
```json
{
  "error": {
    "code": -32001,
    "message": "No presignatures remaining. Return disk to mother for refill."
  }
}
```

---

### Test 5.2: Emergency Reserve Behavior

**Objective:** Verify emergency reserve presigs (if implemented)

**Procedure:**

1. Configure emergency reserve (50 presigs typically)
2. Use presigs until reserve threshold
3. Attempt normal signing

**Expected Results:**
- [ ] Warning issued when approaching reserve
- [ ] Reserve presigs may require confirmation
- [ ] Clear messaging about reserve usage

---

### Test 5.3: Expiration Behavior

**Objective:** Verify behavior when presigs expire

**Procedure:**

1. Create a child disk with short expiration (for testing):
   ```bash
   sigil-mother create-child \
     --presig-count 10 \
     --validity-days 1 \
     --agent-output short_expiry_shares.json
   ```

2. Wait for expiration (or mock system time)

3. Attempt to sign

**Expected Results:**
- [ ] Signing fails after expiration
- [ ] Error clearly indicates expiration
- [ ] `sigil_check_disk` shows expired status

---

### Test 5.4: Max Uses Before Reconciliation

**Objective:** Verify forced reconciliation threshold

**Procedure:**

1. Create disk with low max_uses:
   ```bash
   sigil-mother create-child \
     --presig-count 100 \
     --max-uses-before-reconcile 20 \
     --agent-output limited_uses_shares.json
   ```

2. Sign 20 transactions

3. Attempt 21st signature

**Expected Results:**
- [ ] First 20 signatures succeed
- [ ] 21st signature fails or warns
- [ ] Message indicates reconciliation required

---

## Test Suite 6: Reconciliation

### Test 6.1: Normal Reconciliation (No Anomalies)

**Objective:** Complete successful reconciliation cycle

**Procedure:**

1. Use a child disk for several signatures (e.g., 50)

2. Return disk to mother device

3. Run reconciliation:
   ```bash
   sigil-mother reconcile --disk /dev/fd0
   ```

4. Review reconciliation report

**Expected Results:**
- [ ] All presig statuses consistent
- [ ] Log entry count matches used presigs
- [ ] No anomalies detected
- [ ] "PASSED" status returned
- [ ] Report shows:
  - Total presigs: 1000
  - Used presigs: 50
  - Fresh presigs: 950
  - Voided presigs: 0
  - Anomalies: 0

---

### Test 6.2: Reconciliation with Refill

**Objective:** Complete refill ceremony after reconciliation

**Procedure:**

1. After successful reconciliation:
   ```bash
   sigil-mother refill \
     --disk /dev/fd0 \
     --presig-count 1000 \
     --agent-output child1_refill_shares.json
   ```

2. Import new shares on agent:
   ```bash
   sigil import-child-shares child1_refill_shares.json --replace
   ```

**Expected Results:**
- [ ] New 1000 presignatures generated
- [ ] Disk reset (presig_used = 0)
- [ ] Usage log cleared
- [ ] Expiration timer reset
- [ ] Agent shares replaced

---

### Test 6.3: Detect Missing Log Entry Anomaly

**Objective:** Verify detection of presig used without logging

**Procedure (Manual Tampering Simulation):**

1. Create test disk with 10 presigs
2. Sign 5 transactions normally
3. Manually mark presig #6 as "Used" without log entry:
   ```bash
   # This simulates tampering
   sigil-mother test-tamper \
     --disk /dev/fd0 \
     --mark-used 6 \
     --skip-log
   ```
4. Run reconciliation

**Expected Results:**
- [ ] Reconciliation detects anomaly
- [ ] `MissingLogEntry { presig_index: 6 }` reported
- [ ] Status: "FAILED"
- [ ] Recommendation: Nullify child

---

### Test 6.4: Detect Presig Count Mismatch

**Objective:** Verify detection of header count manipulation

**Procedure (Manual Tampering Simulation):**

1. Create test disk
2. Sign 5 transactions
3. Tamper with header count:
   ```bash
   sigil-mother test-tamper \
     --disk /dev/fd0 \
     --set-header-used-count 3  # Actual is 5
   ```
4. Run reconciliation

**Expected Results:**
- [ ] `CountMismatch { header_count: 3, actual_count: 5 }` detected
- [ ] Reconciliation fails
- [ ] Potential disk clone warning

---

### Test 6.5: Detect Timestamp Anomaly

**Objective:** Verify detection of out-of-order timestamps

**Procedure:**

1. Sign transactions with system clock manipulation
2. Run reconciliation

**Expected Results:**
- [ ] `TimestampAnomaly` detected
- [ ] Indicates potential clock tampering
- [ ] Reconciliation fails

---

### Test 6.6: Detect Orphan Log Entry

**Objective:** Verify detection of log entries for unmarked presigs

**Procedure:**

1. Create scenario where log exists but presig status is Fresh
2. Run reconciliation

**Expected Results:**
- [ ] `OrphanLogEntry` detected
- [ ] Impossible state flagged
- [ ] Critical severity

---

## Test Suite 7: Nullification

### Test 7.1: Manual Nullification

**Objective:** Permanently disable a child disk

**Procedure:**

1. Identify child to nullify:
   ```bash
   sigil-mother list-children
   ```

2. Execute nullification:
   ```bash
   sigil-mother nullify \
     --child-id 7a3f2c1b \
     --reason "ManualRevocation"
   ```

3. Verify on mother side:
   ```bash
   sigil-mother list-children
   # Should show status: Nullified
   ```

4. Delete agent shares:
   ```bash
   sigil delete-child 7a3f2c1b
   ```

5. Attempt to sign with nullified disk

**Expected Results:**
- [ ] Child marked as Nullified in registry
- [ ] Reason recorded
- [ ] Agent shares deleted (zeroized)
- [ ] Signing attempts fail with "Child nullified" error

---

### Test 7.2: Nullification After Anomaly Detection

**Objective:** Nullify child after reconciliation failure

**Procedure:**

1. Trigger reconciliation anomaly (Test 6.3-6.6)

2. When reconciliation fails, nullify:
   ```bash
   sigil-mother nullify \
     --child-id <child_id> \
     --reason "ReconciliationAnomaly" \
     --description "Missing log entry detected for presig 6"
   ```

**Expected Results:**
- [ ] Nullification succeeds
- [ ] Description saved with reason
- [ ] Child cannot be used

---

### Test 7.3: Report Disk Lost/Stolen

**Objective:** Nullify child after physical loss

**Procedure:**

1. Report disk as lost:
   ```bash
   sigil-mother nullify \
     --child-id <child_id> \
     --reason "LostOrStolen" \
     --reported-at "$(date +%s)"
   ```

2. Immediately delete agent shares

**Expected Results:**
- [ ] Child nullified
- [ ] Loss timestamp recorded
- [ ] Agent shares destroyed

**Security Note:** Funds should be moved to different address before nullification if possible.

---

### Test 7.4: Attempt Signing After Nullification

**Objective:** Verify nullified disk cannot sign

**Procedure:**

1. Insert disk that was nullified

2. Attempt check status:
   ```json
   {
     "tool": "sigil_check_disk",
     "arguments": {}
   }
   ```

3. Attempt signing

**Expected Results:**
- [ ] Status shows child is nullified
- [ ] Signing fails with clear error
- [ ] Error includes nullification reason

---

### Test 7.5: Verify Agent Share Deletion

**Objective:** Confirm agent shares are properly destroyed

**Procedure:**

1. After nullification, verify shares deleted:
   ```bash
   sigil list-children
   # Nullified child should not appear or show "shares deleted"
   ```

2. Check agent store directly:
   ```bash
   ls ~/.sigil/agent_store/
   # Child directory should be removed
   ```

**Expected Results:**
- [ ] No trace of presig shares in agent store
- [ ] Sensitive data zeroized before deletion

---

## Test Suite 8: Error Handling & Edge Cases

### Test 8.1: No Disk Inserted

**Objective:** Verify graceful handling when no disk present

**Procedure:**

1. Ensure no Sigil disk inserted
2. Call `sigil_check_disk`
3. Attempt `sigil_sign_evm`

**Expected Results:**
- [ ] `check_disk` returns `{ "detected": false }`
- [ ] `sign_evm` fails with "No disk detected" error
- [ ] Clear instructions to insert disk

---

### Test 8.2: Wrong Disk Inserted

**Objective:** Verify handling of non-Sigil disk

**Procedure:**

1. Insert a regular USB drive or blank floppy
2. Call `sigil_check_disk`

**Expected Results:**
- [ ] Disk detected but not valid Sigil disk
- [ ] "Invalid disk format" or "Magic bytes mismatch"
- [ ] No crash or hang

---

### Test 8.3: Corrupted Disk Header

**Objective:** Verify handling of corrupted disk

**Procedure:**

1. Create test disk
2. Corrupt the header (modify magic bytes)
3. Attempt operations

**Expected Results:**
- [ ] Corruption detected
- [ ] Clear error message
- [ ] No attempt to read presigs

---

### Test 8.4: Invalid Message Hash

**Objective:** Verify input validation

**Procedure:**

1. Attempt to sign with invalid hash:
   ```json
   {
     "tool": "sigil_sign_evm",
     "arguments": {
       "message_hash": "invalid",
       "chain_id": 1,
       "description": "Test"
     }
   }
   ```

2. Try various invalid formats:
   - Too short: `0x1234`
   - Too long: hash longer than 66 chars (e.g. `0x` followed by 65+ hex chars, 67+ total)
   - Non-hex: `0xGGGG...`

**Expected Results:**
- [ ] Clear validation error
- [ ] Specifies expected format
- [ ] No presig consumed

---

### Test 8.5: Daemon Not Running

**Objective:** Verify MCP handling when daemon unavailable

**Procedure:**

1. Stop the sigil-daemon
2. Attempt MCP operations

**Expected Results:**
- [ ] Clear error: "Cannot connect to Sigil daemon"
- [ ] Instructions to start daemon
- [ ] MCP server doesn't crash

---

### Test 8.6: Disk Removed Mid-Operation

**Objective:** Verify handling of disk removal during signing

**Procedure:**

1. Start a signing operation
2. Quickly remove disk (timing-dependent)

**Expected Results:**
- [ ] Operation fails gracefully
- [ ] Presig state consistent (not half-used)
- [ ] Error message indicates disk removal

---

### Test 8.7: Concurrent Signing Requests

**Objective:** Verify serialization of signing operations

**Procedure:**

1. Send multiple signing requests simultaneously

**Expected Results:**
- [ ] Requests serialized (not concurrent)
- [ ] Each request completes or fails atomically
- [ ] No presig index conflicts

---

### Test 8.8: Agent Share Mismatch

**Objective:** Verify detection of mismatched shares

**Procedure:**

1. Create disk A
2. Import shares for disk A
3. Insert disk B (different child)
4. Attempt signing

**Expected Results:**
- [ ] Mismatch detected
- [ ] Error: "Child ID mismatch" or "No shares for this disk"
- [ ] No signing attempted

---

### Test 8.9: R-Point Mismatch Detection

**Objective:** Verify detection of corrupted presig shares

**Procedure:**

1. Manually corrupt agent share R-point (testing tool)
2. Attempt signing

**Expected Results:**
- [ ] R-point mismatch detected
- [ ] Error: "Presig R-point mismatch between cold and agent shares"
- [ ] Presig may be voided

---

## Test Result Recording

### Test Result Template

For each test, record:

```
Test ID: [e.g., 3.3]
Test Name: [e.g., Sign First EVM Transaction]
Date: [YYYY-MM-DD]
Tester: [Name]
Environment: [Agent type, OS, Sigil version]

Prerequisites Met: [Yes/No]
Procedure Followed: [Yes/No/Deviations noted]

Results:
- [ ] Expected Result 1: [Pass/Fail]
- [ ] Expected Result 2: [Pass/Fail]
...

Actual Output: [Copy relevant output]
Screenshots: [Attach if applicable]

Overall Status: [PASS/FAIL]
Notes: [Any observations, issues, or suggestions]
Bugs Filed: [Issue numbers if any]
```

### Summary Report Template

```
=== Sigil E2E Test Summary ===
Date: [YYYY-MM-DD]
Version: [Sigil version]
Tester: [Name]

Test Suite Results:
- Suite 1 (Genesis): [X/Y passed]
- Suite 2 (Child Creation): [X/Y passed]
- Suite 3 (ECDSA Signing): [X/Y passed]
- Suite 4 (FROST Operations): [X/Y passed]
- Suite 5 (Exhaustion): [X/Y passed]
- Suite 6 (Reconciliation): [X/Y passed]
- Suite 7 (Nullification): [X/Y passed]
- Suite 8 (Error Handling): [X/Y passed]

Total: [XX/YY tests passed]

Critical Issues Found:
1. [Description]
2. [Description]

Recommendations:
1. [Suggestion]
2. [Suggestion]

Sign-off: [Approved/Not Approved]
```

---

## Appendix A: Quick Reference Commands

### Mother Device Commands
```bash
# Initialize
sigil-mother init [--ledger]

# Create child
sigil-mother create-child --presig-count N --agent-output FILE [--scheme SCHEME]

# FROST DKG
sigil-mother frost-dkg init --scheme SCHEME --threshold T --parties N --party-id ID
sigil-mother frost-dkg round2 --import FILE
sigil-mother frost-dkg finalize

# Reconciliation
sigil-mother reconcile --disk DEVICE
sigil-mother refill --disk DEVICE --presig-count N --agent-output FILE

# Nullification
sigil-mother nullify --child-id ID --reason REASON

# Utilities
sigil-mother list-children
sigil-mother verify-disk DEVICE
```

### Agent Device Commands
```bash
# Daemon
sudo sigil-daemon --socket PATH

# CLI
sigil status
sigil import-agent-shard --hex HEX_STRING       # Import agent's portion of master key
sigil import-agent-shard --file FILE            # Import from file
sigil import-child-shares FILE [--replace]      # Import child presig shares
sigil list-children
sigil delete-child ID
```

### MCP Tools
```
sigil_check_disk          - Check disk status
sigil_sign_evm            - Sign EVM transaction
sigil_sign_frost          - Sign with FROST scheme
sigil_get_address         - Get blockchain address
sigil_update_tx_hash      - Record transaction hash
sigil_list_schemes        - List supported schemes
sigil_get_presig_count    - Get remaining presigs
```

---

## Appendix B: Troubleshooting

### Common Issues

| Issue | Possible Cause | Solution |
|-------|---------------|----------|
| "No disk detected" | Disk not inserted or not mounted | Check USB connection, verify mount |
| "Invalid disk format" | Wrong disk or corrupted | Use correct Sigil disk, re-verify |
| "Daemon not running" | Service not started | Run `sigil-daemon` |
| "No shares for child" | Child shares not imported | Import child presig shares |
| "Agent shard not loaded" | Agent portion of master key not imported | Import agent shard |
| "Presig exhausted" | All presigs used | Return for refill |
| "Child nullified" | Child was revoked | Create new child disk |
| "R-point mismatch" | Share corruption | Verify shares match, may need new child |

### Log Locations

```
Daemon logs: /var/log/sigil/daemon.log
MCP logs: /var/log/sigil/mcp.log
Agent store: ~/.sigil/agent_store/
Mother store: ~/.sigil-mother/
```

---

## Appendix C: Test Data Generator

For generating test message hashes:

```python
#!/usr/bin/env python3
import hashlib
import secrets

def generate_test_hash():
    """Generate a random 32-byte hash for testing."""
    return "0x" + secrets.token_hex(32)

def generate_eth_tx_hash(nonce, to_addr, value, gas_price, gas_limit, chain_id):
    """Generate a deterministic test hash from tx params."""
    data = f"{nonce}{to_addr}{value}{gas_price}{gas_limit}{chain_id}"
    return "0x" + hashlib.sha256(data.encode()).hexdigest()

# Generate 10 test hashes
print("Test Message Hashes:")
for i in range(10):
    print(f"  TEST_HASH_{i+1}={generate_test_hash()}")
```

---

*Document Version: 1.0*
*Last Updated: 2026-01-18*
*Sigil Version: Compatible with v1.x*
