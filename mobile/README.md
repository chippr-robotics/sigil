# Sigil Mobile App

A secure Flutter mobile application for the Sigil 2-of-2 MPC threshold signing system.

## Overview

Sigil Mobile provides a user-friendly interface for performing cryptographic signing operations using the Sigil MPC system. The app communicates with `sigil-daemon` through the `sigil-bridge` HTTP server, allowing you to sign transactions on various blockchains directly from your mobile device.

## Features

- **Secure PIN Authentication**: 6-digit PIN with lockout protection, biometric support
- **EVM Signing**: Sign transactions for Ethereum and EVM-compatible chains
- **FROST Signing**: Support for Bitcoin (Taproot), Solana, Cosmos, and more
- **Address Management**: View and share your signing addresses with QR codes
- **Offline Mode**: View cached disk status and transaction history when disconnected
- **Transaction History**: Local storage of all signing operations for audit

## Security Features

### PIN Protection
- Minimum 6-digit PIN requirement
- Salted SHA-256 hashing (double-hashed for added security)
- 5 failed attempt lockout (5 minutes)
- Session timeout after 15 minutes of inactivity
- PIN stored in platform secure storage (Keychain/Keystore)

### Biometric Authentication
- Optional fingerprint/face authentication
- Fallback to PIN entry

### Secure Storage
- Android: AES-256-GCM encryption with RSA key wrapping
- iOS: Keychain with first-unlock accessibility

### Screenshot Prevention
- Sensitive screens prevent screenshots (Android)
- Clipboard cleared when leaving secure screens

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Mobile Device                             │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                     Sigil Mobile App                       │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐│  │
│  │  │   PIN Auth  │  │  Dashboard  │  │  Signing Screens    ││  │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘│  │
│  │  ┌─────────────────────────────────────────────────────────│  │
│  │  │          HTTP Client (Dio) + Local Cache               ││  │
│  │  └─────────────────────────────────────────────────────────│  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ HTTP (WiFi/LAN)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Agent Device (e.g., Raspberry Pi)          │
│  ┌──────────────────┐      IPC      ┌──────────────────────┐   │
│  │   sigil-bridge   │ ────────────► │    sigil-daemon      │   │
│  │   (HTTP Server)  │               │    (Signing Engine)  │   │
│  │     :8080        │               │   /tmp/sigil.sock    │   │
│  └──────────────────┘               └──────────────────────┘   │
│                                              │                  │
│                                              │ Floppy Disk      │
│                                              ▼                  │
│                                     ┌────────────────┐          │
│                                     │  SIGIL Disk    │          │
│                                     │  (Presigs)     │          │
│                                     └────────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

## Prerequisites

1. **Agent Device Setup**
   - Device with floppy drive (or USB floppy)
   - `sigil-daemon` running
   - `sigil-bridge` running
   - Both devices on same network

2. **Development Requirements**
   - Flutter SDK 3.2.0+
   - Dart SDK 3.2.0+
   - Android Studio / Xcode (for native builds)

## Installation

### From Source

```bash
# Clone the repository
cd sigil/mobile

# Get dependencies
flutter pub get

# Generate code (freezed, riverpod_generator, etc.)
flutter pub run build_runner build --delete-conflicting-outputs

# Run on device/emulator
flutter run

# Build release APK
flutter build apk --release

# Build release iOS
flutter build ios --release
```

### Quick Start

1. Install the app on your mobile device
2. Create a 6-digit PIN when prompted
3. Go to Settings > Daemon Connection
4. Enter your agent device's IP and port (e.g., `http://192.168.1.100:8080`)
5. Test the connection
6. Insert your Sigil disk into the agent device
7. Return to Dashboard - you should see your disk status

## Usage Guide

### Initial Setup

1. **Set PIN**: Create a secure 6-digit PIN
2. **Configure Connection**: Enter the sigil-bridge URL
3. **Optional**: Enable biometric authentication

### Signing a Transaction

1. Ensure disk is inserted and detected (green status)
2. Tap "Sign EVM" or "Sign FROST" from dashboard
3. Select the network/scheme
4. Enter the message hash (32-byte hex)
5. Add a description for audit purposes
6. Review and tap "Sign"
7. Copy the signature from the result

### Viewing Addresses

1. Navigate to "Addresses" screen
2. Tap any address to view QR code
3. Use "Copy" or "Share" to export

### Transaction History

1. Tap the history icon in the app bar
2. View all past signing operations
3. History is stored locally for offline access

## Configuration

### Environment Variables

None required. All configuration is done through the app UI.

### Build Configuration

Edit `pubspec.yaml` for:
- App name and version
- Dependencies
- Asset paths

Edit platform-specific files for:
- Android: `android/app/build.gradle`
- iOS: `ios/Runner/Info.plist`

## Offline Mode

The app gracefully handles offline scenarios:

- **Cached Disk Status**: Last known disk state shown with "Offline" indicator
- **Transaction History**: Always available locally
- **Signing Operations**: Require daemon connection (shows warning)
- **Settings**: All settings work offline

## Supported Chains

### EVM (ECDSA)
- Ethereum (1)
- Polygon (137)
- Arbitrum One (42161)
- Optimism (10)
- Base (8453)
- BNB Chain (56)
- Avalanche (43114)
- Testnets: Sepolia, Mumbai, etc.

### FROST
- Bitcoin Taproot (BIP-340)
- Solana (Ed25519)
- Cosmos/Cosmos Hub (Ed25519)
- Zcash Shielded (Ristretto255)

## Troubleshooting

### Connection Issues
- Verify both devices are on the same network
- Check firewall allows port 8080
- Verify sigil-daemon is running: `curl http://<ip>:8080/health`
- Check sigil-bridge logs for errors

### Signing Failures
- Ensure disk is inserted and valid
- Check presignatures remaining > 0
- Verify disk hasn't expired
- Check scheme compatibility (ECDSA disk for EVM)

### PIN Lockout
- Wait for lockout timer to expire (5 minutes)
- If needed, reinstall app to reset (loses all data)

## Development

### Project Structure

```
mobile/
├── lib/
│   ├── main.dart              # App entry point
│   ├── app_router.dart        # Navigation routes
│   ├── core/
│   │   ├── api/               # Daemon client
│   │   ├── auth/              # PIN authentication
│   │   ├── models/            # Data models
│   │   ├── storage/           # Secure & local storage
│   │   └── utils/             # Utilities
│   ├── features/
│   │   ├── auth/              # PIN screens
│   │   ├── dashboard/         # Main dashboard
│   │   ├── signing/           # EVM & FROST signing
│   │   ├── addresses/         # Address management
│   │   └── settings/          # App settings
│   └── shared/
│       ├── theme/             # App theming
│       └── widgets/           # Reusable widgets
├── assets/                    # Images, fonts, icons
├── android/                   # Android native code
├── ios/                       # iOS native code
└── pubspec.yaml               # Flutter dependencies
```

### State Management

Uses Riverpod for:
- Authentication state
- Daemon connection status
- Disk status (with caching)
- Settings

### Testing

```bash
# Run unit tests
flutter test

# Run integration tests
flutter test integration_test/
```

## Security Considerations

1. **PIN Security**: PIN is never stored in plaintext
2. **Network Security**: Use private/secure network only
3. **Physical Security**: Signing requires physical disk insertion
4. **Audit Trail**: All operations logged on disk for reconciliation

## License

MIT License - see LICENSE file for details.

## Contributing

See CONTRIBUTING.md for contribution guidelines.

## Support

For issues and feature requests, please use GitHub Issues.
