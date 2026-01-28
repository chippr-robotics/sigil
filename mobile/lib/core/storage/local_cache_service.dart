import 'dart:convert';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:sigil_mobile/core/models/models.dart';

/// Local cache service for offline support.
///
/// Caches non-sensitive data locally for offline access:
/// - Last known disk status
/// - Addresses
/// - Chain configurations
/// - Transaction history
class LocalCacheService {
  static const _diskStatusKey = 'cache_disk_status';
  static const _addressesKey = 'cache_addresses';
  static const _lastSyncKey = 'cache_last_sync';
  static const _transactionHistoryKey = 'cache_tx_history';
  static const _pendingSignaturesKey = 'cache_pending_sigs';

  SharedPreferences? _prefs;

  Future<SharedPreferences> get _preferences async {
    _prefs ??= await SharedPreferences.getInstance();
    return _prefs!;
  }

  /// Cache disk status for offline display
  Future<void> cacheDiskStatus(DiskStatus status) async {
    final prefs = await _preferences;
    await prefs.setString(_diskStatusKey, jsonEncode(status.toJson()));
    await prefs.setString(_lastSyncKey, DateTime.now().toIso8601String());
  }

  /// Get cached disk status
  Future<DiskStatus?> getCachedDiskStatus() async {
    final prefs = await _preferences;
    final json = prefs.getString(_diskStatusKey);
    if (json == null) return null;
    try {
      return DiskStatus.fromJson(jsonDecode(json));
    } catch (_) {
      return null;
    }
  }

  /// Cache addresses for offline display
  Future<void> cacheAddresses(Map<String, AddressInfo> addresses) async {
    final prefs = await _preferences;
    final json = addresses.map((k, v) => MapEntry(k, v.address));
    await prefs.setString(_addressesKey, jsonEncode(json));
  }

  /// Get cached addresses
  Future<Map<String, String>> getCachedAddresses() async {
    final prefs = await _preferences;
    final json = prefs.getString(_addressesKey);
    if (json == null) return {};
    try {
      final decoded = jsonDecode(json) as Map<String, dynamic>;
      return decoded.map((k, v) => MapEntry(k, v.toString()));
    } catch (_) {
      return {};
    }
  }

  /// Get last sync time
  Future<DateTime?> getLastSyncTime() async {
    final prefs = await _preferences;
    final str = prefs.getString(_lastSyncKey);
    if (str == null) return null;
    return DateTime.tryParse(str);
  }

  /// Save transaction to history
  Future<void> addTransaction(TransactionRecord tx) async {
    final prefs = await _preferences;
    final history = await getTransactionHistory();
    history.insert(0, tx);
    // Keep only last 100 transactions
    final trimmed = history.take(100).toList();
    await prefs.setString(
      _transactionHistoryKey,
      jsonEncode(trimmed.map((t) => t.toJson()).toList()),
    );
  }

  /// Get transaction history
  Future<List<TransactionRecord>> getTransactionHistory() async {
    final prefs = await _preferences;
    final json = prefs.getString(_transactionHistoryKey);
    if (json == null) return [];
    try {
      final list = jsonDecode(json) as List;
      return list.map((j) => TransactionRecord.fromJson(j)).toList();
    } catch (_) {
      return [];
    }
  }

  /// Add pending signature (for offline queuing)
  Future<void> addPendingSignature(PendingSignature sig) async {
    final prefs = await _preferences;
    final pending = await getPendingSignatures();
    pending.add(sig);
    await prefs.setString(
      _pendingSignaturesKey,
      jsonEncode(pending.map((s) => s.toJson()).toList()),
    );
  }

  /// Get pending signatures
  Future<List<PendingSignature>> getPendingSignatures() async {
    final prefs = await _preferences;
    final json = prefs.getString(_pendingSignaturesKey);
    if (json == null) return [];
    try {
      final list = jsonDecode(json) as List;
      return list.map((j) => PendingSignature.fromJson(j)).toList();
    } catch (_) {
      return [];
    }
  }

  /// Remove pending signature
  Future<void> removePendingSignature(String id) async {
    final prefs = await _preferences;
    final pending = await getPendingSignatures();
    pending.removeWhere((s) => s.id == id);
    await prefs.setString(
      _pendingSignaturesKey,
      jsonEncode(pending.map((s) => s.toJson()).toList()),
    );
  }

  /// Clear all cached data
  Future<void> clearCache() async {
    final prefs = await _preferences;
    await prefs.remove(_diskStatusKey);
    await prefs.remove(_addressesKey);
    await prefs.remove(_lastSyncKey);
  }
}

/// Transaction record for history
class TransactionRecord {
  final String id;
  final String signature;
  final int presigIndex;
  final String? txHash;
  final int? chainId;
  final String? chainName;
  final String description;
  final DateTime timestamp;
  final String scheme;

  const TransactionRecord({
    required this.id,
    required this.signature,
    required this.presigIndex,
    this.txHash,
    this.chainId,
    this.chainName,
    required this.description,
    required this.timestamp,
    required this.scheme,
  });

  factory TransactionRecord.fromJson(Map<String, dynamic> json) {
    return TransactionRecord(
      id: json['id'] as String,
      signature: json['signature'] as String,
      presigIndex: json['presig_index'] as int,
      txHash: json['tx_hash'] as String?,
      chainId: json['chain_id'] as int?,
      chainName: json['chain_name'] as String?,
      description: json['description'] as String,
      timestamp: DateTime.parse(json['timestamp'] as String),
      scheme: json['scheme'] as String,
    );
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        'signature': signature,
        'presig_index': presigIndex,
        'tx_hash': txHash,
        'chain_id': chainId,
        'chain_name': chainName,
        'description': description,
        'timestamp': timestamp.toIso8601String(),
        'scheme': scheme,
      };
}

/// Pending signature for offline queuing
class PendingSignature {
  final String id;
  final String messageHash;
  final int? chainId;
  final String? scheme;
  final String description;
  final DateTime createdAt;

  const PendingSignature({
    required this.id,
    required this.messageHash,
    this.chainId,
    this.scheme,
    required this.description,
    required this.createdAt,
  });

  factory PendingSignature.fromJson(Map<String, dynamic> json) {
    return PendingSignature(
      id: json['id'] as String,
      messageHash: json['message_hash'] as String,
      chainId: json['chain_id'] as int?,
      scheme: json['scheme'] as String?,
      description: json['description'] as String,
      createdAt: DateTime.parse(json['created_at'] as String),
    );
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        'message_hash': messageHash,
        'chain_id': chainId,
        'scheme': scheme,
        'description': description,
        'created_at': createdAt.toIso8601String(),
      };
}
