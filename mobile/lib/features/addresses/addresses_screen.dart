import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:qr_flutter/qr_flutter.dart';
import 'package:sigil_mobile/core/api/daemon_client.dart';
import 'package:sigil_mobile/core/auth/auth_provider.dart';
import 'package:sigil_mobile/core/models/models.dart';
import 'package:sigil_mobile/core/storage/local_cache_service.dart';
import 'package:sigil_mobile/features/dashboard/dashboard_screen.dart';
import 'package:sigil_mobile/shared/theme/app_theme.dart';

/// Addresses provider
final addressesProvider =
    FutureProvider.autoDispose<Map<String, AddressInfo>>((ref) async {
  final client = ref.watch(daemonClientProvider);
  final connectionStatus = ref.watch(daemonConnectionStatusProvider);

  if (connectionStatus != DaemonConnectionStatus.connected) {
    return {};
  }

  final addresses = <String, AddressInfo>{};

  // Fetch addresses for different formats
  final formats = [
    ('evm', null),
    ('bitcoin', null),
    ('solana', null),
    ('cosmos', 'cosmos'),
  ];

  for (final (format, prefix) in formats) {
    try {
      final address = await client.getAddress(
        format: format,
        cosmosPrefix: prefix,
      );
      addresses[format] = address;
    } catch (_) {
      // Skip if format not supported
    }
  }

  return addresses;
});

/// Addresses screen
class AddressesScreen extends ConsumerStatefulWidget {
  const AddressesScreen({super.key});

  @override
  ConsumerState<AddressesScreen> createState() => _AddressesScreenState();
}

class _AddressesScreenState extends ConsumerState<AddressesScreen> {
  @override
  void initState() {
    super.initState();
    ref.read(authStateProvider.notifier).refreshSession();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final diskStatusAsync = ref.watch(combinedDiskStatusProvider);
    final connectionStatus = ref.watch(daemonConnectionStatusProvider);
    final isOffline = connectionStatus != DaemonConnectionStatus.connected;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Addresses'),
      ),
      body: diskStatusAsync.when(
        data: (data) {
          if (!data.status.detected) {
            return _buildNoDiskView(context);
          }
          return _buildAddressList(context, data.status, isOffline);
        },
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text('Error: $e')),
      ),
    );
  }

  Widget _buildNoDiskView(BuildContext context) {
    final theme = Theme.of(context);

    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.disc_full_outlined,
              size: 64,
              color: Colors.grey.shade400,
            ),
            const SizedBox(height: 16),
            Text(
              'No Disk Detected',
              style: theme.textTheme.titleLarge,
            ),
            const SizedBox(height: 8),
            Text(
              'Insert your Sigil disk to view addresses',
              style: theme.textTheme.bodyMedium?.copyWith(
                color: Colors.grey,
              ),
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildAddressList(
    BuildContext context,
    DiskStatus diskStatus,
    bool isOffline,
  ) {
    final theme = Theme.of(context);
    final addressesAsync = ref.watch(addressesProvider);

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Disk info header
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Row(
                children: [
                  Icon(
                    Icons.verified,
                    color: AppTheme.successColor,
                  ),
                  const SizedBox(width: 12),
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Disk: ${diskStatus.childId ?? "Unknown"}',
                        style: theme.textTheme.titleMedium,
                      ),
                      Text(
                        'Scheme: ${_formatScheme(diskStatus.scheme)}',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: Colors.grey,
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 24),

          // Offline warning
          if (isOffline)
            Container(
              margin: const EdgeInsets.only(bottom: 16),
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: AppTheme.warningColor.withAlpha(25),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Row(
                children: [
                  const Icon(Icons.cloud_off, color: AppTheme.warningColor),
                  const SizedBox(width: 12),
                  const Expanded(
                    child: Text(
                      'Offline - showing cached addresses',
                      style: TextStyle(color: AppTheme.warningColor),
                    ),
                  ),
                ],
              ),
            ),

          Text('Your Addresses', style: theme.textTheme.titleLarge),
          const SizedBox(height: 16),

          // Address list
          addressesAsync.when(
            data: (addresses) {
              if (addresses.isEmpty && isOffline) {
                return _buildOfflinePlaceholder(context);
              }
              if (addresses.isEmpty) {
                return _buildNoAddresses(context);
              }
              return Column(
                children: addresses.entries.map((entry) {
                  return _AddressCard(
                    format: entry.key,
                    address: entry.value,
                  );
                }).toList(),
              );
            },
            loading: () => const Center(
              child: Padding(
                padding: EdgeInsets.all(32),
                child: CircularProgressIndicator(),
              ),
            ),
            error: (e, _) => _buildOfflinePlaceholder(context),
          ),

          const SizedBox(height: 24),

          // Public key section
          if (diskStatus.publicKey != null) ...[
            Text('Public Key', style: theme.textTheme.titleLarge),
            const SizedBox(height: 16),
            _PublicKeyCard(publicKey: diskStatus.publicKey!),
          ],
        ],
      ),
    );
  }

  Widget _buildOfflinePlaceholder(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          children: [
            Icon(
              Icons.cloud_off,
              size: 48,
              color: Colors.grey.shade400,
            ),
            const SizedBox(height: 12),
            Text(
              'Connect to daemon to view addresses',
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: Colors.grey,
                  ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildNoAddresses(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          children: [
            Icon(
              Icons.account_balance_wallet_outlined,
              size: 48,
              color: Colors.grey.shade400,
            ),
            const SizedBox(height: 12),
            Text(
              'No addresses available',
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: Colors.grey,
                  ),
            ),
          ],
        ),
      ),
    );
  }

  String _formatScheme(String? scheme) {
    switch (scheme) {
      case 'ecdsa':
        return 'ECDSA (EVM)';
      case 'taproot':
        return 'FROST Taproot';
      case 'ed25519':
        return 'FROST Ed25519';
      case 'ristretto255':
        return 'FROST Ristretto255';
      default:
        return scheme ?? 'Unknown';
    }
  }
}

class _AddressCard extends StatelessWidget {
  final String format;
  final AddressInfo address;

  const _AddressCard({
    required this.format,
    required this.address,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    final (icon, label) = switch (format) {
      'evm' => (Icons.currency_exchange, 'Ethereum / EVM'),
      'bitcoin' => (Icons.currency_bitcoin, 'Bitcoin'),
      'solana' => (Icons.blur_circular, 'Solana'),
      'cosmos' => (Icons.language, 'Cosmos'),
      _ => (Icons.account_balance_wallet, format.toUpperCase()),
    };

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: InkWell(
        onTap: () => _showAddressDetails(context),
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: theme.colorScheme.primary.withAlpha(25),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Icon(icon, color: theme.colorScheme.primary),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(label, style: theme.textTheme.titleMedium),
                    const SizedBox(height: 4),
                    Text(
                      _truncateAddress(address.address),
                      style: theme.textTheme.bodySmall?.copyWith(
                        fontFamily: 'monospace',
                        color: Colors.grey,
                      ),
                    ),
                  ],
                ),
              ),
              IconButton(
                icon: const Icon(Icons.copy),
                onPressed: () => _copyAddress(context),
                tooltip: 'Copy address',
              ),
            ],
          ),
        ),
      ),
    );
  }

  String _truncateAddress(String addr) {
    if (addr.length <= 20) return addr;
    return '${addr.substring(0, 10)}...${addr.substring(addr.length - 8)}';
  }

  void _copyAddress(BuildContext context) {
    Clipboard.setData(ClipboardData(text: address.address));
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('Address copied to clipboard')),
    );
  }

  void _showAddressDetails(BuildContext context) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (context) => _AddressDetailsSheet(
        format: format,
        address: address,
      ),
    );
  }
}

class _AddressDetailsSheet extends StatelessWidget {
  final String format;
  final AddressInfo address;

  const _AddressDetailsSheet({
    required this.format,
    required this.address,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    final label = switch (format) {
      'evm' => 'Ethereum / EVM Address',
      'bitcoin' => 'Bitcoin Address',
      'solana' => 'Solana Address',
      'cosmos' => 'Cosmos Address',
      _ => '${format.toUpperCase()} Address',
    };

    return DraggableScrollableSheet(
      initialChildSize: 0.6,
      minChildSize: 0.4,
      maxChildSize: 0.9,
      expand: false,
      builder: (context, scrollController) => SingleChildScrollView(
        controller: scrollController,
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            Container(
              width: 40,
              height: 4,
              decoration: BoxDecoration(
                color: Colors.grey.shade300,
                borderRadius: BorderRadius.circular(2),
              ),
            ),
            const SizedBox(height: 24),
            Text(label, style: theme.textTheme.titleLarge),
            const SizedBox(height: 24),
            // QR Code
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: Colors.white,
                borderRadius: BorderRadius.circular(12),
              ),
              child: QrImageView(
                data: address.address,
                version: QrVersions.auto,
                size: 200,
              ),
            ),
            const SizedBox(height: 24),
            // Address
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: Colors.grey.shade100,
                borderRadius: BorderRadius.circular(8),
              ),
              child: SelectableText(
                address.address,
                style: const TextStyle(
                  fontFamily: 'monospace',
                  fontSize: 14,
                ),
                textAlign: TextAlign.center,
              ),
            ),
            const SizedBox(height: 16),
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                OutlinedButton.icon(
                  onPressed: () {
                    Clipboard.setData(ClipboardData(text: address.address));
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('Address copied')),
                    );
                  },
                  icon: const Icon(Icons.copy),
                  label: const Text('Copy'),
                ),
                const SizedBox(width: 12),
                OutlinedButton.icon(
                  onPressed: () {
                    // Share functionality would go here
                    Navigator.pop(context);
                  },
                  icon: const Icon(Icons.share),
                  label: const Text('Share'),
                ),
              ],
            ),
            const SizedBox(height: 24),
            // Details
            _buildDetailRow('Format', address.format),
            _buildDetailRow('Scheme', address.scheme),
            _buildDetailRow('Child ID', address.childId),
          ],
        ),
      ),
    );
  }

  Widget _buildDetailRow(String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: const TextStyle(color: Colors.grey)),
          Text(value, style: const TextStyle(fontWeight: FontWeight.w500)),
        ],
      ),
    );
  }
}

class _PublicKeyCard extends StatelessWidget {
  final String publicKey;

  const _PublicKeyCard({required this.publicKey});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.key, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Text('Raw Public Key', style: theme.textTheme.titleMedium),
                const Spacer(),
                IconButton(
                  icon: const Icon(Icons.copy),
                  onPressed: () {
                    Clipboard.setData(ClipboardData(text: publicKey));
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('Public key copied')),
                    );
                  },
                  tooltip: 'Copy',
                ),
              ],
            ),
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: Colors.grey.shade100,
                borderRadius: BorderRadius.circular(8),
              ),
              child: SelectableText(
                publicKey,
                style: const TextStyle(
                  fontFamily: 'monospace',
                  fontSize: 12,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
