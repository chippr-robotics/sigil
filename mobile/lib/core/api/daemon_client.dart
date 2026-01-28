import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logger/logger.dart';
import 'package:sigil_mobile/core/models/models.dart';
import 'package:sigil_mobile/core/storage/secure_storage_service.dart';
import 'package:sigil_mobile/core/auth/auth_provider.dart';

/// HTTP client for communicating with sigil-daemon via HTTP bridge.
///
/// The mobile app connects to an HTTP bridge server that proxies requests
/// to the sigil-daemon's Unix socket/named pipe IPC interface.
class DaemonClient {
  final Dio _dio;
  final Logger _logger = Logger();
  String _baseUrl;

  DaemonClient({String? baseUrl})
      : _baseUrl = baseUrl ?? 'http://localhost:8080',
        _dio = Dio(
          BaseOptions(
            connectTimeout: const Duration(seconds: 10),
            receiveTimeout: const Duration(seconds: 30),
            headers: {
              'Content-Type': 'application/json',
            },
          ),
        );

  void updateBaseUrl(String url) {
    _baseUrl = url;
  }

  String get baseUrl => _baseUrl;

  /// Ping daemon to check connection
  Future<String> ping() async {
    try {
      final response = await _dio.post(
        '$_baseUrl/api/ping',
        data: {'type': 'Ping'},
      );
      return response.data['version'] as String? ?? 'unknown';
    } catch (e) {
      _logger.e('Ping failed: $e');
      rethrow;
    }
  }

  /// Get disk status
  Future<DiskStatus> getDiskStatus() async {
    try {
      final response = await _dio.post(
        '$_baseUrl/api/disk-status',
        data: {'type': 'GetDiskStatus'},
      );
      return DiskStatus.fromJson(response.data);
    } on DioException catch (e) {
      _logger.e('Get disk status failed: $e');
      if (e.type == DioExceptionType.connectionError ||
          e.type == DioExceptionType.connectionTimeout) {
        throw DaemonConnectionException('Cannot connect to daemon');
      }
      rethrow;
    }
  }

  /// Get presignature count
  Future<PresigCount> getPresigCount() async {
    try {
      final response = await _dio.post(
        '$_baseUrl/api/presig-count',
        data: {'type': 'GetPresigCount'},
      );
      return PresigCount.fromJson(response.data);
    } catch (e) {
      _logger.e('Get presig count failed: $e');
      rethrow;
    }
  }

  /// Sign EVM message hash
  Future<SignResult> signEvm({
    required String messageHash,
    required int chainId,
    required String description,
  }) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/api/sign',
        data: {
          'type': 'Sign',
          'message_hash': messageHash,
          'chain_id': chainId,
          'description': description,
        },
      );

      if (response.data['type'] == 'Error') {
        throw DaemonException(response.data['message'] as String);
      }

      return SignResult.fromJson(response.data);
    } catch (e) {
      _logger.e('Sign EVM failed: $e');
      rethrow;
    }
  }

  /// Sign with FROST scheme
  Future<SignResult> signFrost({
    required String scheme,
    required String messageHash,
    required String description,
  }) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/api/sign-frost',
        data: {
          'scheme': scheme,
          'message_hash': messageHash,
          'description': description,
        },
      );

      if (response.data['type'] == 'Error') {
        throw DaemonException(response.data['message'] as String);
      }

      return SignResult.fromJson(response.data);
    } catch (e) {
      _logger.e('Sign FROST failed: $e');
      rethrow;
    }
  }

  /// Get address in specified format
  Future<AddressInfo> getAddress({
    String? scheme,
    required String format,
    String? cosmosPrefix,
  }) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/api/address',
        data: {
          if (scheme != null) 'scheme': scheme,
          'format': format,
          if (cosmosPrefix != null) 'cosmos_prefix': cosmosPrefix,
        },
      );

      if (response.data['type'] == 'Error') {
        throw DaemonException(response.data['message'] as String);
      }

      return AddressInfo.fromJson(response.data);
    } catch (e) {
      _logger.e('Get address failed: $e');
      rethrow;
    }
  }

  /// Update transaction hash for audit log
  Future<void> updateTxHash({
    required int presigIndex,
    required String txHash,
  }) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/api/update-tx-hash',
        data: {
          'type': 'UpdateTxHash',
          'presig_index': presigIndex,
          'tx_hash': txHash,
        },
      );

      if (response.data['type'] == 'Error') {
        throw DaemonException(response.data['message'] as String);
      }
    } catch (e) {
      _logger.e('Update tx hash failed: $e');
      rethrow;
    }
  }

  /// List all child disk IDs
  Future<List<String>> listChildren() async {
    try {
      final response = await _dio.post(
        '$_baseUrl/api/list-children',
        data: {'type': 'ListChildren'},
      );
      return (response.data['child_ids'] as List).cast<String>();
    } catch (e) {
      _logger.e('List children failed: $e');
      rethrow;
    }
  }

  /// List supported signature schemes
  Future<List<SignatureScheme>> listSchemes() async {
    try {
      final response = await _dio.get('$_baseUrl/api/schemes');
      final schemes = response.data['schemes'] as List;
      return schemes.map((s) => SignatureScheme.fromJson(s)).toList();
    } catch (e) {
      _logger.e('List schemes failed: $e');
      rethrow;
    }
  }

  /// Import agent shard
  Future<void> importAgentShard(String shardHex) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/api/import-agent-shard',
        data: {
          'type': 'ImportAgentShard',
          'agent_shard_hex': shardHex,
        },
      );

      if (response.data['type'] == 'Error') {
        throw DaemonException(response.data['message'] as String);
      }
    } catch (e) {
      _logger.e('Import agent shard failed: $e');
      rethrow;
    }
  }

  /// Import child shares
  Future<void> importChildShares({
    required String sharesJson,
    bool replace = false,
  }) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/api/import-child-shares',
        data: {
          'type': 'ImportChildShares',
          'shares_json': sharesJson,
          'replace': replace,
        },
      );

      if (response.data['type'] == 'Error') {
        throw DaemonException(response.data['message'] as String);
      }
    } catch (e) {
      _logger.e('Import child shares failed: $e');
      rethrow;
    }
  }
}

/// Daemon communication exceptions
class DaemonException implements Exception {
  final String message;

  DaemonException(this.message);

  @override
  String toString() => message;
}

class DaemonConnectionException extends DaemonException {
  DaemonConnectionException(super.message);
}

/// Daemon client provider
final daemonClientProvider = Provider<DaemonClient>((ref) {
  return DaemonClient();
});

/// Daemon connection status provider
final daemonConnectionStatusProvider =
    StateNotifierProvider<DaemonConnectionNotifier, DaemonConnectionStatus>((ref) {
  final client = ref.watch(daemonClientProvider);
  final storage = ref.watch(secureStorageProvider);
  return DaemonConnectionNotifier(client, storage);
});

class DaemonConnectionNotifier extends StateNotifier<DaemonConnectionStatus> {
  final DaemonClient _client;
  final SecureStorageService _storage;

  DaemonConnectionNotifier(this._client, this._storage)
      : super(DaemonConnectionStatus.disconnected) {
    _initialize();
  }

  Future<void> _initialize() async {
    final url = await _storage.getDaemonUrl();
    if (url != null) {
      _client.updateBaseUrl(url);
      await connect();
    }
  }

  Future<void> connect() async {
    state = DaemonConnectionStatus.connecting;
    try {
      await _client.ping();
      state = DaemonConnectionStatus.connected;
    } catch (e) {
      state = DaemonConnectionStatus.error;
    }
  }

  Future<void> updateUrl(String url) async {
    await _storage.setDaemonUrl(url);
    _client.updateBaseUrl(url);
    await connect();
  }
}

/// Disk status provider (auto-refreshes)
final diskStatusProvider = FutureProvider.autoDispose<DiskStatus>((ref) async {
  final client = ref.watch(daemonClientProvider);
  final status = ref.watch(daemonConnectionStatusProvider);

  if (status != DaemonConnectionStatus.connected) {
    return DiskStatus.notDetected();
  }

  return await client.getDiskStatus();
});

/// Presig count provider
final presigCountProvider = FutureProvider.autoDispose<PresigCount?>((ref) async {
  final client = ref.watch(daemonClientProvider);
  final diskStatus = await ref.watch(diskStatusProvider.future);

  if (!diskStatus.detected) {
    return null;
  }

  return await client.getPresigCount();
});
