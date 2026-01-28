/// Data models for Sigil mobile app

/// Disk status from sigil-daemon
class DiskStatus {
  final bool detected;
  final String? childId;
  final String? scheme;
  final int? presigsRemaining;
  final int? presigsTotal;
  final int? daysUntilExpiry;
  final bool? isValid;
  final String? publicKey;

  const DiskStatus({
    required this.detected,
    this.childId,
    this.scheme,
    this.presigsRemaining,
    this.presigsTotal,
    this.daysUntilExpiry,
    this.isValid,
    this.publicKey,
  });

  factory DiskStatus.notDetected() => const DiskStatus(detected: false);

  factory DiskStatus.fromJson(Map<String, dynamic> json) {
    return DiskStatus(
      detected: json['detected'] as bool? ?? false,
      childId: json['child_id'] as String?,
      scheme: json['scheme'] as String?,
      presigsRemaining: json['presigs_remaining'] as int?,
      presigsTotal: json['presigs_total'] as int?,
      daysUntilExpiry: json['days_until_expiry'] as int?,
      isValid: json['is_valid'] as bool?,
      publicKey: json['public_key'] as String?,
    );
  }

  Map<String, dynamic> toJson() => {
    'detected': detected,
    'child_id': childId,
    'scheme': scheme,
    'presigs_remaining': presigsRemaining,
    'presigs_total': presigsTotal,
    'days_until_expiry': daysUntilExpiry,
    'is_valid': isValid,
    'public_key': publicKey,
  };

  double? get presigPercentage {
    if (presigsRemaining == null || presigsTotal == null || presigsTotal == 0) {
      return null;
    }
    return presigsRemaining! / presigsTotal!;
  }

  bool get isLowPresigs => (presigsRemaining ?? 0) < 100;
  bool get isExpiringSoon => (daysUntilExpiry ?? 999) < 7;
}

/// Presignature count info
class PresigCount {
  final int remaining;
  final int total;
  final double percentage;

  const PresigCount({
    required this.remaining,
    required this.total,
    required this.percentage,
  });

  factory PresigCount.fromJson(Map<String, dynamic> json) {
    return PresigCount(
      remaining: json['remaining'] as int,
      total: json['total'] as int,
      percentage: (json['percentage'] as num).toDouble(),
    );
  }

  bool get isLow => remaining < 100;
  bool get isCritical => remaining < 10;
}

/// Signing result
class SignResult {
  final String signature;
  final int presigIndex;
  final String proofHash;
  final int? v;
  final String? r;
  final String? s;
  final int? chainId;
  final String? messageHash;

  const SignResult({
    required this.signature,
    required this.presigIndex,
    required this.proofHash,
    this.v,
    this.r,
    this.s,
    this.chainId,
    this.messageHash,
  });

  factory SignResult.fromJson(Map<String, dynamic> json) {
    return SignResult(
      signature: json['signature'] as String,
      presigIndex: json['presig_index'] as int,
      proofHash: json['proof_hash'] as String,
      v: json['v'] as int?,
      r: json['r'] as String?,
      s: json['s'] as String?,
      chainId: json['chain_id'] as int?,
      messageHash: json['message_hash'] as String?,
    );
  }
}

/// Address information
class AddressInfo {
  final String address;
  final String format;
  final String scheme;
  final String publicKey;
  final String childId;

  const AddressInfo({
    required this.address,
    required this.format,
    required this.scheme,
    required this.publicKey,
    required this.childId,
  });

  factory AddressInfo.fromJson(Map<String, dynamic> json) {
    return AddressInfo(
      address: json['address'] as String,
      format: json['format'] as String,
      scheme: json['scheme'] as String,
      publicKey: json['public_key'] as String,
      childId: json['child_id'] as String,
    );
  }
}

/// Signature scheme info
class SignatureScheme {
  final String name;
  final String description;
  final List<String> chains;

  const SignatureScheme({
    required this.name,
    required this.description,
    required this.chains,
  });

  factory SignatureScheme.fromJson(Map<String, dynamic> json) {
    return SignatureScheme(
      name: json['name'] as String,
      description: json['description'] as String,
      chains: (json['chains'] as List).cast<String>(),
    );
  }
}

/// Supported EVM chain
class EvmChain {
  final int chainId;
  final String name;
  final String symbol;
  final bool isTestnet;

  const EvmChain({
    required this.chainId,
    required this.name,
    required this.symbol,
    this.isTestnet = false,
  });

  static const List<EvmChain> mainnetChains = [
    EvmChain(chainId: 1, name: 'Ethereum', symbol: 'ETH'),
    EvmChain(chainId: 137, name: 'Polygon', symbol: 'MATIC'),
    EvmChain(chainId: 42161, name: 'Arbitrum One', symbol: 'ETH'),
    EvmChain(chainId: 10, name: 'Optimism', symbol: 'ETH'),
    EvmChain(chainId: 8453, name: 'Base', symbol: 'ETH'),
    EvmChain(chainId: 56, name: 'BNB Chain', symbol: 'BNB'),
    EvmChain(chainId: 43114, name: 'Avalanche', symbol: 'AVAX'),
  ];

  static const List<EvmChain> testnetChains = [
    EvmChain(chainId: 11155111, name: 'Sepolia', symbol: 'ETH', isTestnet: true),
    EvmChain(chainId: 80001, name: 'Mumbai', symbol: 'MATIC', isTestnet: true),
    EvmChain(chainId: 421613, name: 'Arbitrum Goerli', symbol: 'ETH', isTestnet: true),
  ];

  static EvmChain? fromChainId(int chainId) {
    try {
      return [...mainnetChains, ...testnetChains]
          .firstWhere((c) => c.chainId == chainId);
    } catch (_) {
      return null;
    }
  }
}

/// FROST scheme types
enum FrostScheme {
  taproot('taproot', 'Bitcoin Taproot (BIP-340)'),
  ed25519('ed25519', 'Ed25519 (Solana, Cosmos)'),
  ristretto255('ristretto255', 'Ristretto255 (Zcash)');

  final String value;
  final String displayName;

  const FrostScheme(this.value, this.displayName);

  static FrostScheme? fromString(String value) {
    try {
      return FrostScheme.values.firstWhere((s) => s.value == value);
    } catch (_) {
      return null;
    }
  }
}

/// Address format types
enum AddressFormat {
  hex('hex', 'Raw Hex'),
  evm('evm', 'EVM (Ethereum)'),
  bitcoin('bitcoin', 'Bitcoin'),
  solana('solana', 'Solana'),
  cosmos('cosmos', 'Cosmos');

  final String value;
  final String displayName;

  const AddressFormat(this.value, this.displayName);
}

/// Daemon connection status
enum DaemonConnectionStatus {
  disconnected,
  connecting,
  connected,
  error,
}
