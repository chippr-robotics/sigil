# Sigil Mobile App - Demo Walkthrough

This document provides a step-by-step demonstration of the Sigil Mobile app's capabilities.

## Prerequisites

Before running the demo, ensure you have:

1. **Agent Device** (e.g., Raspberry Pi, Linux laptop)
   - sigil-daemon installed and running
   - sigil-bridge installed and running on port 8080
   - A configured Sigil floppy disk with presignatures

2. **Mobile Device**
   - Sigil Mobile app installed
   - Connected to same network as agent device

## Demo Scenario

We'll walk through a complete workflow: setup, configuration, viewing status, and signing a transaction.

---

## Part 1: Initial Setup

### Step 1: Launch the App

When you first launch Sigil Mobile, you'll be greeted with the PIN setup screen.

```
┌─────────────────────────────┐
│           [Shield Icon]      │
│                              │
│            Sigil             │
│     Secure MPC Signing       │
│                              │
│     Create a 6-digit PIN     │
│  This PIN will protect your  │
│     signing operations       │
│                              │
│     [  ] [  ] [  ] [  ] [  ] [  ]     │
│                              │
│         [Show PIN]           │
│                              │
│  🔒 Your PIN is securely     │
│     stored on device         │
└─────────────────────────────┘
```

### Step 2: Create PIN

1. Enter a 6-digit PIN (e.g., `123456`)
2. Confirm the PIN by entering it again
3. You'll be redirected to the main dashboard

---

## Part 2: Configure Daemon Connection

### Step 3: Access Settings

From the dashboard, tap the **Settings** icon (gear) in the top right.

### Step 4: Configure Daemon Connection

1. Tap "Daemon Connection"
2. Enter your sigil-bridge URL:
   ```
   http://192.168.1.100:8080
   ```
3. Tap "Test Connection"
4. You should see: "Connected! Daemon version: 0.3.0"
5. Tap "Save and Connect"

```
┌─────────────────────────────┐
│   ← Daemon Connection        │
├─────────────────────────────┤
│                              │
│   Current Status             │
│   [Cloud ✓] Connected        │
│                              │
│   HTTP Bridge URL            │
│   ┌──────────────────────┐  │
│   │ http://192.168.1.100 │  │
│   │ :8080                │  │
│   └──────────────────────┘  │
│                              │
│   [Test Connection]          │
│                              │
│   ✓ Connected! Daemon v0.3.0│
│                              │
│   [  Save and Connect  ]     │
│                              │
└─────────────────────────────┘
```

---

## Part 3: View Disk Status

### Step 5: Return to Dashboard

After connecting, you'll see the disk status card on the main dashboard.

**Without disk:**
```
┌─────────────────────────────┐
│  [Disk Icon]                 │
│                              │
│     No Disk Detected         │
│  Insert your Sigil disk to   │
│     begin signing            │
└─────────────────────────────┘
```

### Step 6: Insert Sigil Disk

Insert your Sigil floppy disk into the agent device's floppy drive.

**With disk detected:**
```
┌─────────────────────────────┐
│  [✓] Sigil Disk    sigil_7a3f │
│                              │
│  Scheme      ECDSA (EVM)     │
│                              │
│  Presignatures               │
│  847 / 1000                  │
│  [████████████░░░]  84.7%    │
│                              │
│  Expires in    12 days       │
│                              │
└─────────────────────────────┘
```

---

## Part 4: View Addresses

### Step 7: Navigate to Addresses

Tap "Addresses" from the quick actions on the dashboard.

```
┌─────────────────────────────┐
│    ← Addresses               │
├─────────────────────────────┤
│                              │
│  Disk: sigil_7a3f2c1b        │
│  Scheme: ECDSA (EVM)         │
│                              │
│  ┌───────────────────────┐   │
│  │ [ETH] Ethereum / EVM  │   │
│  │ 0x742d35Cc...5f12345 [📋]│
│  └───────────────────────┘   │
│                              │
│  ┌───────────────────────┐   │
│  │ [₿] Bitcoin           │   │
│  │ bc1p5cyxnu...kedrcr [📋]│
│  └───────────────────────┘   │
│                              │
└─────────────────────────────┘
```

### Step 8: View Address Details

Tap an address to see the full address and QR code.

```
┌─────────────────────────────┐
│   Ethereum / EVM Address     │
│                              │
│   ┌─────────────────────┐   │
│   │     [QR CODE]        │   │
│   │                      │   │
│   │                      │   │
│   └─────────────────────┘   │
│                              │
│  0x742d35Cc6634C0532925a3   │
│  b844Bc9e7595f12345         │
│                              │
│   [Copy]     [Share]         │
│                              │
│  Format: evm                 │
│  Scheme: ecdsa               │
│  Child ID: 7a3f2c1b          │
└─────────────────────────────┘
```

---

## Part 5: Sign an EVM Transaction

### Step 9: Navigate to Sign EVM

From the dashboard, tap "Sign EVM".

### Step 10: Fill Signing Form

1. **Network**: Select "Ethereum" (Chain ID: 1)
2. **Message Hash**: Enter your transaction hash
   ```
   0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
   ```
3. **Description**: Enter "Transfer 0.1 ETH to vitalik.eth"

```
┌─────────────────────────────┐
│    ← Sign EVM Transaction    │
├─────────────────────────────┤
│                              │
│  Network                     │
│  ┌──────────────────────┐   │
│  │ Ethereum         ▼    │   │
│  └──────────────────────┘   │
│  [ ] Show testnets           │
│                              │
│  Message Hash                │
│  ┌──────────────────────┐   │
│  │ 0x1234567890abcdef   │   │
│  │ ...                   │   │
│  └──────────────────────┘   │
│                              │
│  Description                 │
│  ┌──────────────────────┐   │
│  │ Transfer 0.1 ETH to  │   │
│  │ vitalik.eth          │   │
│  └──────────────────────┘   │
│                              │
│  [    Sign Transaction    ]  │
│                              │
└─────────────────────────────┘
```

### Step 11: Review Signature Result

After signing, you'll see the result:

```
┌─────────────────────────────┐
│  ✓ Signature Created         │
├─────────────────────────────┤
│                              │
│  Presig Index    #153        │
│                              │
│  ─────────────────────────   │
│  Signature                   │
│  0xaabbccdd1122334455...    │
│  ...6677889900aabbcc [📋]    │
│                              │
│  v: 27                       │
│                              │
│  r:                          │
│  0x1234567890abcdef... [📋]  │
│                              │
│  s:                          │
│  0xfedcba0987654321... [📋]  │
│                              │
│  ─────────────────────────   │
│  Proof Hash                  │
│  0x111122223333444... [📋]   │
│                              │
└─────────────────────────────┘
```

---

## Part 6: Sign with FROST (Bitcoin Taproot)

### Step 12: Navigate to Sign FROST

From the dashboard, tap "Sign FROST".

### Step 13: Select Scheme and Fill Form

1. **Scheme**: Select "Bitcoin Taproot (BIP-340)"
2. **Message Hash**: Enter your Bitcoin transaction sighash
3. **Description**: Enter "BTC transfer to bc1q..."

### Step 14: Review Result

Similar to EVM, you'll receive a 64-byte Schnorr signature.

---

## Part 7: Offline Mode Demo

### Step 15: Disconnect from Network

Turn off WiFi on your mobile device.

### Step 16: View Cached Data

Return to the dashboard:

```
┌─────────────────────────────┐
│  [Cloud ✗] Offline mode      │
├─────────────────────────────┤
│                              │
│  ┌───────────────────────┐   │
│  │ [✓] Sigil Disk [Offline]│
│  │                        │   │
│  │ sigil_7a3f2c1b         │   │
│  │ 847/1000 presigs       │   │
│  │                        │   │
│  │ Last synced: 5 min ago │   │
│  └───────────────────────┘   │
│                              │
│  Quick Actions               │
│  ┌─────┐  ┌─────┐            │
│  │Sign │  │Sign │  (disabled)│
│  │EVM  │  │FROST│            │
│  └─────┘  └─────┘            │
│                              │
│  Recent Activity             │
│  [Shows cached tx history]   │
│                              │
└─────────────────────────────┘
```

Note: Signing is disabled offline, but you can still:
- View cached disk status
- View transaction history
- View cached addresses
- Access settings

---

## Part 8: Security Features Demo

### Step 17: Session Timeout

Leave the app idle for 15 minutes. When you return:

```
┌─────────────────────────────┐
│                              │
│       Welcome Back           │
│    Enter your PIN to         │
│         continue             │
│                              │
│   [  ] [  ] [  ] [  ] [  ] [  ]     │
│                              │
│      [Use Biometrics]        │
└─────────────────────────────┘
```

### Step 18: Failed Attempt Lockout

Enter wrong PIN 5 times:

```
┌─────────────────────────────┐
│       [Lock Clock Icon]      │
│                              │
│     Too Many Attempts        │
│                              │
│  Please wait 4:32 before     │
│       trying again.          │
│                              │
└─────────────────────────────┘
```

### Step 19: Change PIN

1. Go to Settings > Change PIN
2. Enter current PIN
3. Enter new PIN
4. Confirm new PIN

---

## Summary

This demo covered:

| Feature | Status |
|---------|--------|
| PIN Setup | ✓ |
| Daemon Connection | ✓ |
| Disk Status Display | ✓ |
| Address Viewing | ✓ |
| EVM Signing | ✓ |
| FROST Signing | ✓ |
| Offline Mode | ✓ |
| Security Features | ✓ |

## Next Steps

1. Review transaction history for audit purposes
2. Set up biometric authentication for convenience
3. Familiarize yourself with different signing schemes
4. Practice the full signing workflow before production use

## Troubleshooting

If you encounter issues:

1. Check daemon connection in Settings
2. Verify disk is properly inserted
3. Check presignature availability
4. Review logs on agent device

For more help, see the main README.md documentation.
