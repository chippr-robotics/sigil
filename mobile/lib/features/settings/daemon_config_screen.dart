import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:sigil_mobile/core/api/daemon_client.dart';
import 'package:sigil_mobile/core/auth/auth_provider.dart';
import 'package:sigil_mobile/core/storage/secure_storage_service.dart';
import 'package:sigil_mobile/shared/theme/app_theme.dart';

/// Daemon configuration screen
class DaemonConfigScreen extends ConsumerStatefulWidget {
  const DaemonConfigScreen({super.key});

  @override
  ConsumerState<DaemonConfigScreen> createState() => _DaemonConfigScreenState();
}

class _DaemonConfigScreenState extends ConsumerState<DaemonConfigScreen> {
  final _urlController = TextEditingController();
  final _formKey = GlobalKey<FormState>();
  bool _isLoading = false;
  bool _isTesting = false;
  String? _testResult;

  @override
  void initState() {
    super.initState();
    ref.read(authStateProvider.notifier).refreshSession();
    _loadCurrentUrl();
  }

  Future<void> _loadCurrentUrl() async {
    final storage = ref.read(secureStorageProvider);
    final url = await storage.getDaemonUrl();
    if (url != null) {
      _urlController.text = url;
    } else {
      _urlController.text = 'http://192.168.1.100:8080';
    }
  }

  @override
  void dispose() {
    _urlController.dispose();
    super.dispose();
  }

  Future<void> _testConnection() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() {
      _isTesting = true;
      _testResult = null;
    });

    try {
      final client = DaemonClient(baseUrl: _urlController.text.trim());
      final version = await client.ping();
      setState(() {
        _testResult = 'Connected! Daemon version: $version';
        _isTesting = false;
      });
    } catch (e) {
      setState(() {
        _testResult = 'Connection failed: $e';
        _isTesting = false;
      });
    }
  }

  Future<void> _saveAndConnect() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() => _isLoading = true);

    try {
      await ref
          .read(daemonConnectionStatusProvider.notifier)
          .updateUrl(_urlController.text.trim());

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Settings saved')),
        );
        Navigator.pop(context);
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Error: $e')),
        );
      }
    } finally {
      setState(() => _isLoading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final connectionStatus = ref.watch(daemonConnectionStatusProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Daemon Connection'),
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16),
        child: Form(
          key: _formKey,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // Current status
              Card(
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Row(
                    children: [
                      Container(
                        padding: const EdgeInsets.all(10),
                        decoration: BoxDecoration(
                          color: _getStatusColor(connectionStatus).withAlpha(25),
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: Icon(
                          _getStatusIcon(connectionStatus),
                          color: _getStatusColor(connectionStatus),
                        ),
                      ),
                      const SizedBox(width: 16),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              'Current Status',
                              style: theme.textTheme.titleMedium,
                            ),
                            Text(
                              _getStatusText(connectionStatus),
                              style: theme.textTheme.bodySmall?.copyWith(
                                color: _getStatusColor(connectionStatus),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 24),

              // Configuration
              Text(
                'HTTP Bridge URL',
                style: theme.textTheme.titleMedium,
              ),
              const SizedBox(height: 8),
              TextFormField(
                controller: _urlController,
                decoration: const InputDecoration(
                  hintText: 'http://192.168.1.100:8080',
                  helperText: 'URL of the sigil-bridge HTTP server',
                  prefixIcon: Icon(Icons.link),
                ),
                keyboardType: TextInputType.url,
                validator: (value) {
                  if (value == null || value.isEmpty) {
                    return 'URL is required';
                  }
                  final uri = Uri.tryParse(value);
                  if (uri == null || !uri.hasScheme || !uri.hasAuthority) {
                    return 'Invalid URL format';
                  }
                  return null;
                },
              ),
              const SizedBox(height: 16),

              // Test connection button
              OutlinedButton.icon(
                onPressed: _isTesting ? null : _testConnection,
                icon: _isTesting
                    ? const SizedBox(
                        width: 18,
                        height: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.wifi_find),
                label: Text(_isTesting ? 'Testing...' : 'Test Connection'),
              ),

              // Test result
              if (_testResult != null) ...[
                const SizedBox(height: 16),
                Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: _testResult!.startsWith('Connected')
                        ? AppTheme.successColor.withAlpha(25)
                        : AppTheme.errorColor.withAlpha(25),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Row(
                    children: [
                      Icon(
                        _testResult!.startsWith('Connected')
                            ? Icons.check_circle
                            : Icons.error,
                        color: _testResult!.startsWith('Connected')
                            ? AppTheme.successColor
                            : AppTheme.errorColor,
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          _testResult!,
                          style: TextStyle(
                            color: _testResult!.startsWith('Connected')
                                ? AppTheme.successColor
                                : AppTheme.errorColor,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
              const SizedBox(height: 24),

              // Save button
              ElevatedButton(
                onPressed: _isLoading ? null : _saveAndConnect,
                child: _isLoading
                    ? const SizedBox(
                        width: 20,
                        height: 20,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Text('Save and Connect'),
              ),
              const SizedBox(height: 32),

              // Help section
              Card(
                color: AppTheme.infoColor.withAlpha(25),
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Icon(Icons.info_outline, color: AppTheme.infoColor),
                          const SizedBox(width: 8),
                          Text(
                            'Setup Instructions',
                            style: theme.textTheme.titleMedium?.copyWith(
                              color: AppTheme.infoColor,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 12),
                      _buildHelpItem(
                        '1.',
                        'Start sigil-daemon on your agent device',
                      ),
                      _buildHelpItem(
                        '2.',
                        'Start sigil-bridge HTTP server',
                      ),
                      _buildHelpItem(
                        '3.',
                        'Ensure both devices are on the same network',
                      ),
                      _buildHelpItem(
                        '4.',
                        'Enter the bridge server URL above',
                      ),
                      const SizedBox(height: 12),
                      Text(
                        'Example: If your agent device IP is 192.168.1.100 and the bridge runs on port 8080, use http://192.168.1.100:8080',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: Colors.grey.shade700,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildHelpItem(String number, String text) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(number, style: const TextStyle(fontWeight: FontWeight.bold)),
          const SizedBox(width: 8),
          Expanded(child: Text(text)),
        ],
      ),
    );
  }

  Color _getStatusColor(DaemonConnectionStatus status) {
    return switch (status) {
      DaemonConnectionStatus.connected => AppTheme.successColor,
      DaemonConnectionStatus.connecting => AppTheme.warningColor,
      DaemonConnectionStatus.disconnected => Colors.grey,
      DaemonConnectionStatus.error => AppTheme.errorColor,
    };
  }

  IconData _getStatusIcon(DaemonConnectionStatus status) {
    return switch (status) {
      DaemonConnectionStatus.connected => Icons.cloud_done,
      DaemonConnectionStatus.connecting => Icons.cloud_sync,
      DaemonConnectionStatus.disconnected => Icons.cloud_off,
      DaemonConnectionStatus.error => Icons.cloud_off,
    };
  }

  String _getStatusText(DaemonConnectionStatus status) {
    return switch (status) {
      DaemonConnectionStatus.connected => 'Connected to daemon',
      DaemonConnectionStatus.connecting => 'Connecting...',
      DaemonConnectionStatus.disconnected => 'Not connected',
      DaemonConnectionStatus.error => 'Connection error',
    };
  }
}
