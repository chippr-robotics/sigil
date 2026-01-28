import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:sigil_mobile/core/api/daemon_client.dart';
import 'package:sigil_mobile/core/auth/auth_provider.dart';
import 'package:sigil_mobile/core/models/models.dart';
import 'package:sigil_mobile/core/storage/local_cache_service.dart';
import 'package:sigil_mobile/shared/theme/app_theme.dart';
import 'package:sigil_mobile/shared/widgets/connection_status_bar.dart';
import 'package:sigil_mobile/shared/widgets/disk_status_card.dart';
import 'package:sigil_mobile/shared/widgets/quick_action_card.dart';

/// Local cache provider
final localCacheProvider = Provider<LocalCacheService>((ref) {
  return LocalCacheService();
});

/// Combined disk status provider with offline fallback
final combinedDiskStatusProvider = FutureProvider.autoDispose<DiskStatusWithMeta>((ref) async {
  final client = ref.watch(daemonClientProvider);
  final cache = ref.watch(localCacheProvider);
  final connectionStatus = ref.watch(daemonConnectionStatusProvider);

  DiskStatus? status;
  bool isFromCache = false;
  DateTime? lastSync;

  if (connectionStatus == DaemonConnectionStatus.connected) {
    try {
      status = await client.getDiskStatus();
      await cache.cacheDiskStatus(status);
      lastSync = DateTime.now();
    } catch (_) {
      // Fall back to cache
    }
  }

  if (status == null) {
    status = await cache.getCachedDiskStatus();
    lastSync = await cache.getLastSyncTime();
    isFromCache = true;
  }

  return DiskStatusWithMeta(
    status: status ?? DiskStatus.notDetected(),
    isFromCache: isFromCache,
    lastSync: lastSync,
  );
});

class DiskStatusWithMeta {
  final DiskStatus status;
  final bool isFromCache;
  final DateTime? lastSync;

  DiskStatusWithMeta({
    required this.status,
    required this.isFromCache,
    this.lastSync,
  });
}

/// Main dashboard screen
class DashboardScreen extends ConsumerStatefulWidget {
  const DashboardScreen({super.key});

  @override
  ConsumerState<DashboardScreen> createState() => _DashboardScreenState();
}

class _DashboardScreenState extends ConsumerState<DashboardScreen> {
  @override
  void initState() {
    super.initState();
    // Refresh session on activity
    ref.read(authStateProvider.notifier).refreshSession();
  }

  Future<void> _refresh() async {
    ref.invalidate(combinedDiskStatusProvider);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final diskStatusAsync = ref.watch(combinedDiskStatusProvider);
    final connectionStatus = ref.watch(daemonConnectionStatusProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Sigil'),
        actions: [
          IconButton(
            icon: const Icon(Icons.history),
            onPressed: () => _showTransactionHistory(context),
            tooltip: 'Transaction History',
          ),
          IconButton(
            icon: const Icon(Icons.settings),
            onPressed: () => context.push('/settings'),
            tooltip: 'Settings',
          ),
        ],
      ),
      body: RefreshIndicator(
        onRefresh: _refresh,
        child: SingleChildScrollView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // Connection status bar
              ConnectionStatusBar(status: connectionStatus),
              const SizedBox(height: 16),

              // Disk status card
              diskStatusAsync.when(
                data: (data) => DiskStatusCard(
                  status: data.status,
                  isFromCache: data.isFromCache,
                  lastSync: data.lastSync,
                ),
                loading: () => const _LoadingCard(),
                error: (e, _) => _ErrorCard(message: e.toString()),
              ),
              const SizedBox(height: 24),

              // Quick actions
              Text(
                'Quick Actions',
                style: theme.textTheme.titleLarge,
              ),
              const SizedBox(height: 12),
              _buildQuickActions(context, diskStatusAsync),
              const SizedBox(height: 24),

              // Recent activity
              Text(
                'Recent Activity',
                style: theme.textTheme.titleLarge,
              ),
              const SizedBox(height: 12),
              _buildRecentActivity(context),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildQuickActions(
    BuildContext context,
    AsyncValue<DiskStatusWithMeta> diskStatusAsync,
  ) {
    final isDiskReady = diskStatusAsync.valueOrNull?.status.detected == true &&
        diskStatusAsync.valueOrNull?.status.isValid == true;

    return Wrap(
      spacing: 12,
      runSpacing: 12,
      children: [
        QuickActionCard(
          icon: Icons.edit_document,
          title: 'Sign EVM',
          subtitle: 'Ethereum & EVM chains',
          enabled: isDiskReady,
          onTap: () => context.push('/sign/evm'),
        ),
        QuickActionCard(
          icon: Icons.currency_bitcoin,
          title: 'Sign FROST',
          subtitle: 'Bitcoin, Solana, etc.',
          enabled: isDiskReady,
          onTap: () => context.push('/sign/frost'),
        ),
        QuickActionCard(
          icon: Icons.account_balance_wallet,
          title: 'Addresses',
          subtitle: 'View signing addresses',
          enabled: true,
          onTap: () => context.push('/addresses'),
        ),
        QuickActionCard(
          icon: Icons.qr_code_scanner,
          title: 'Scan QR',
          subtitle: 'Scan transaction data',
          enabled: isDiskReady,
          onTap: () => _showQrScanner(context),
        ),
      ],
    );
  }

  Widget _buildRecentActivity(BuildContext context) {
    return FutureBuilder<List<TransactionRecord>>(
      future: ref.read(localCacheProvider).getTransactionHistory(),
      builder: (context, snapshot) {
        if (!snapshot.hasData || snapshot.data!.isEmpty) {
          return Card(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                children: [
                  Icon(
                    Icons.history,
                    size: 48,
                    color: Colors.grey.shade400,
                  ),
                  const SizedBox(height: 12),
                  Text(
                    'No recent activity',
                    style: TextStyle(color: Colors.grey.shade600),
                  ),
                ],
              ),
            ),
          );
        }

        final transactions = snapshot.data!.take(5).toList();
        return Card(
          child: ListView.separated(
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            itemCount: transactions.length,
            separatorBuilder: (_, __) => const Divider(height: 1),
            itemBuilder: (context, index) {
              final tx = transactions[index];
              return ListTile(
                leading: CircleAvatar(
                  backgroundColor: Theme.of(context).colorScheme.primary.withAlpha(25),
                  child: Icon(
                    tx.scheme == 'ecdsa'
                        ? Icons.currency_exchange
                        : Icons.currency_bitcoin,
                    color: Theme.of(context).colorScheme.primary,
                  ),
                ),
                title: Text(
                  tx.description,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                subtitle: Text(
                  '${tx.chainName ?? tx.scheme} • Presig #${tx.presigIndex}',
                ),
                trailing: Text(
                  _formatTime(tx.timestamp),
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              );
            },
          ),
        );
      },
    );
  }

  String _formatTime(DateTime time) {
    final now = DateTime.now();
    final diff = now.difference(time);

    if (diff.inMinutes < 1) return 'Just now';
    if (diff.inHours < 1) return '${diff.inMinutes}m ago';
    if (diff.inDays < 1) return '${diff.inHours}h ago';
    if (diff.inDays < 7) return '${diff.inDays}d ago';
    return '${time.day}/${time.month}';
  }

  void _showTransactionHistory(BuildContext context) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (context) => DraggableScrollableSheet(
        initialChildSize: 0.7,
        minChildSize: 0.5,
        maxChildSize: 0.95,
        expand: false,
        builder: (context, scrollController) => Column(
          children: [
            Container(
              padding: const EdgeInsets.all(16),
              child: Row(
                children: [
                  Text(
                    'Transaction History',
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                  const Spacer(),
                  IconButton(
                    icon: const Icon(Icons.close),
                    onPressed: () => Navigator.pop(context),
                  ),
                ],
              ),
            ),
            const Divider(height: 1),
            Expanded(
              child: FutureBuilder<List<TransactionRecord>>(
                future: ref.read(localCacheProvider).getTransactionHistory(),
                builder: (context, snapshot) {
                  if (!snapshot.hasData || snapshot.data!.isEmpty) {
                    return const Center(
                      child: Text('No transaction history'),
                    );
                  }
                  return ListView.builder(
                    controller: scrollController,
                    itemCount: snapshot.data!.length,
                    itemBuilder: (context, index) {
                      final tx = snapshot.data![index];
                      return _TransactionListItem(tx: tx);
                    },
                  );
                },
              ),
            ),
          ],
        ),
      ),
    );
  }

  void _showQrScanner(BuildContext context) {
    // QR scanner implementation would go here
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('QR Scanner coming soon')),
    );
  }
}

class _LoadingCard extends StatelessWidget {
  const _LoadingCard();

  @override
  Widget build(BuildContext context) {
    return const Card(
      child: Padding(
        padding: EdgeInsets.all(32),
        child: Center(child: CircularProgressIndicator()),
      ),
    );
  }
}

class _ErrorCard extends StatelessWidget {
  final String message;

  const _ErrorCard({required this.message});

  @override
  Widget build(BuildContext context) {
    return Card(
      color: Theme.of(context).colorScheme.error.withAlpha(25),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            Icon(
              Icons.error_outline,
              color: Theme.of(context).colorScheme.error,
            ),
            const SizedBox(width: 12),
            Expanded(child: Text(message)),
          ],
        ),
      ),
    );
  }
}

class _TransactionListItem extends StatelessWidget {
  final TransactionRecord tx;

  const _TransactionListItem({required this.tx});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return ListTile(
      leading: CircleAvatar(
        backgroundColor: theme.colorScheme.primary.withAlpha(25),
        child: Icon(
          tx.scheme == 'ecdsa'
              ? Icons.currency_exchange
              : Icons.currency_bitcoin,
          color: theme.colorScheme.primary,
        ),
      ),
      title: Text(tx.description),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('${tx.chainName ?? tx.scheme} • Presig #${tx.presigIndex}'),
          if (tx.txHash != null)
            Text(
              'TX: ${tx.txHash!.substring(0, 10)}...${tx.txHash!.substring(tx.txHash!.length - 8)}',
              style: theme.textTheme.bodySmall,
            ),
        ],
      ),
      trailing: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          Text(
            '${tx.timestamp.hour}:${tx.timestamp.minute.toString().padLeft(2, '0')}',
            style: theme.textTheme.bodySmall,
          ),
          Text(
            '${tx.timestamp.day}/${tx.timestamp.month}/${tx.timestamp.year}',
            style: theme.textTheme.bodySmall?.copyWith(
              color: Colors.grey,
            ),
          ),
        ],
      ),
      isThreeLine: tx.txHash != null,
    );
  }
}
