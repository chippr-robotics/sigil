import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:sigil_mobile/core/api/daemon_client.dart';
import 'package:sigil_mobile/core/auth/auth_provider.dart';
import 'package:sigil_mobile/core/storage/local_cache_service.dart';
import 'package:sigil_mobile/features/dashboard/dashboard_screen.dart';
import 'package:sigil_mobile/shared/theme/app_theme.dart';

/// Settings screen
class SettingsScreen extends ConsumerStatefulWidget {
  const SettingsScreen({super.key});

  @override
  ConsumerState<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends ConsumerState<SettingsScreen> {
  @override
  void initState() {
    super.initState();
    ref.read(authStateProvider.notifier).refreshSession();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final authState = ref.watch(authStateProvider);
    final connectionStatus = ref.watch(daemonConnectionStatusProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Settings'),
      ),
      body: ListView(
        children: [
          // Connection section
          _buildSectionHeader(context, 'Connection'),
          ListTile(
            leading: Icon(
              connectionStatus == DaemonConnectionStatus.connected
                  ? Icons.cloud_done
                  : Icons.cloud_off,
              color: connectionStatus == DaemonConnectionStatus.connected
                  ? AppTheme.successColor
                  : Colors.grey,
            ),
            title: const Text('Daemon Connection'),
            subtitle: Text(_getConnectionStatusText(connectionStatus)),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => context.push('/settings/daemon'),
          ),
          const Divider(),

          // Security section
          _buildSectionHeader(context, 'Security'),
          ListTile(
            leading: const Icon(Icons.lock),
            title: const Text('Change PIN'),
            subtitle: const Text('Update your security PIN'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => _showChangePinDialog(context),
          ),
          SwitchListTile(
            secondary: const Icon(Icons.fingerprint),
            title: const Text('Biometric Login'),
            subtitle: Text(
              authState.isBiometricAvailable
                  ? 'Use fingerprint or face to unlock'
                  : 'Not available on this device',
            ),
            value: authState.isBiometricEnabled,
            onChanged: authState.isBiometricAvailable
                ? (value) {
                    ref.read(authStateProvider.notifier).setBiometricEnabled(value);
                  }
                : null,
          ),
          ListTile(
            leading: const Icon(Icons.timer),
            title: const Text('Session Timeout'),
            subtitle: const Text('15 minutes of inactivity'),
            enabled: false,
          ),
          const Divider(),

          // Data section
          _buildSectionHeader(context, 'Data'),
          FutureBuilder<DateTime?>(
            future: ref.read(localCacheProvider).getLastSyncTime(),
            builder: (context, snapshot) {
              return ListTile(
                leading: const Icon(Icons.sync),
                title: const Text('Last Sync'),
                subtitle: Text(
                  snapshot.data != null
                      ? _formatDateTime(snapshot.data!)
                      : 'Never synced',
                ),
                trailing: OutlinedButton(
                  onPressed: connectionStatus == DaemonConnectionStatus.connected
                      ? () {
                          ref.invalidate(combinedDiskStatusProvider);
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(content: Text('Syncing...')),
                          );
                        }
                      : null,
                  child: const Text('Sync Now'),
                ),
              );
            },
          ),
          FutureBuilder<List<TransactionRecord>>(
            future: ref.read(localCacheProvider).getTransactionHistory(),
            builder: (context, snapshot) {
              final count = snapshot.data?.length ?? 0;
              return ListTile(
                leading: const Icon(Icons.history),
                title: const Text('Transaction History'),
                subtitle: Text('$count transactions stored locally'),
              );
            },
          ),
          ListTile(
            leading: Icon(Icons.delete_outline, color: AppTheme.warningColor),
            title: Text('Clear Cache', style: TextStyle(color: AppTheme.warningColor)),
            subtitle: const Text('Remove locally cached data'),
            onTap: () => _showClearCacheDialog(context),
          ),
          const Divider(),

          // About section
          _buildSectionHeader(context, 'About'),
          ListTile(
            leading: const Icon(Icons.info_outline),
            title: const Text('Version'),
            subtitle: const Text('1.0.0'),
          ),
          ListTile(
            leading: const Icon(Icons.description_outlined),
            title: const Text('Licenses'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => showLicensePage(
              context: context,
              applicationName: 'Sigil Mobile',
              applicationVersion: '1.0.0',
            ),
          ),
          const Divider(),

          // Danger zone
          _buildSectionHeader(
            context,
            'Danger Zone',
            color: theme.colorScheme.error,
          ),
          ListTile(
            leading: Icon(Icons.logout, color: theme.colorScheme.error),
            title: Text('Lock App', style: TextStyle(color: theme.colorScheme.error)),
            subtitle: const Text('Require PIN to access again'),
            onTap: () => _logout(context),
          ),
          ListTile(
            leading: Icon(Icons.delete_forever, color: theme.colorScheme.error),
            title: Text('Wipe All Data', style: TextStyle(color: theme.colorScheme.error)),
            subtitle: const Text('Delete all app data and reset'),
            onTap: () => _showWipeDataDialog(context),
          ),
          const SizedBox(height: 32),
        ],
      ),
    );
  }

  Widget _buildSectionHeader(BuildContext context, String title, {Color? color}) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
      child: Text(
        title,
        style: Theme.of(context).textTheme.titleSmall?.copyWith(
              color: color ?? Colors.grey,
              fontWeight: FontWeight.w600,
            ),
      ),
    );
  }

  String _getConnectionStatusText(DaemonConnectionStatus status) {
    return switch (status) {
      DaemonConnectionStatus.connected => 'Connected',
      DaemonConnectionStatus.connecting => 'Connecting...',
      DaemonConnectionStatus.disconnected => 'Not connected',
      DaemonConnectionStatus.error => 'Connection error',
    };
  }

  String _formatDateTime(DateTime dt) {
    return '${dt.day}/${dt.month}/${dt.year} at ${dt.hour}:${dt.minute.toString().padLeft(2, '0')}';
  }

  void _showChangePinDialog(BuildContext context) {
    final currentPinController = TextEditingController();
    final newPinController = TextEditingController();
    final confirmPinController = TextEditingController();

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Change PIN'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: currentPinController,
              decoration: const InputDecoration(labelText: 'Current PIN'),
              keyboardType: TextInputType.number,
              obscureText: true,
              maxLength: 6,
            ),
            TextField(
              controller: newPinController,
              decoration: const InputDecoration(labelText: 'New PIN'),
              keyboardType: TextInputType.number,
              obscureText: true,
              maxLength: 6,
            ),
            TextField(
              controller: confirmPinController,
              decoration: const InputDecoration(labelText: 'Confirm New PIN'),
              keyboardType: TextInputType.number,
              obscureText: true,
              maxLength: 6,
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () async {
              if (newPinController.text != confirmPinController.text) {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('PINs do not match')),
                );
                return;
              }
              try {
                await ref.read(authStateProvider.notifier).changePin(
                      currentPinController.text,
                      newPinController.text,
                    );
                if (context.mounted) {
                  Navigator.pop(context);
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('PIN changed successfully')),
                  );
                }
              } catch (e) {
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(content: Text('Error: $e')),
                  );
                }
              }
            },
            child: const Text('Change'),
          ),
        ],
      ),
    );
  }

  void _showClearCacheDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Clear Cache?'),
        content: const Text(
          'This will remove locally cached data including disk status and transaction history. Your PIN and settings will be preserved.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () async {
              await ref.read(localCacheProvider).clearCache();
              if (context.mounted) {
                Navigator.pop(context);
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('Cache cleared')),
                );
              }
            },
            child: const Text('Clear'),
          ),
        ],
      ),
    );
  }

  void _showWipeDataDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Wipe All Data?'),
        content: const Text(
          'This will permanently delete all app data including your PIN, settings, and cached data. This action cannot be undone.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            style: ElevatedButton.styleFrom(
              backgroundColor: Theme.of(context).colorScheme.error,
            ),
            onPressed: () async {
              await ref.read(authStateProvider.notifier).wipeAllData();
              await ref.read(localCacheProvider).clearCache();
              if (context.mounted) {
                Navigator.pop(context);
                context.go('/auth/setup');
              }
            },
            child: const Text('Wipe Data'),
          ),
        ],
      ),
    );
  }

  void _logout(BuildContext context) {
    ref.read(authStateProvider.notifier).logout();
    context.go('/auth/pin');
  }
}
