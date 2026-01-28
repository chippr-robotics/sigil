import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:sigil_mobile/core/api/daemon_client.dart';
import 'package:sigil_mobile/core/auth/auth_provider.dart';
import 'package:sigil_mobile/core/models/models.dart';
import 'package:sigil_mobile/core/storage/local_cache_service.dart';
import 'package:sigil_mobile/features/dashboard/dashboard_screen.dart';
import 'package:sigil_mobile/shared/theme/app_theme.dart';

/// EVM signing screen
class SignEvmScreen extends ConsumerStatefulWidget {
  const SignEvmScreen({super.key});

  @override
  ConsumerState<SignEvmScreen> createState() => _SignEvmScreenState();
}

class _SignEvmScreenState extends ConsumerState<SignEvmScreen> {
  final _formKey = GlobalKey<FormState>();
  final _messageHashController = TextEditingController();
  final _descriptionController = TextEditingController();

  EvmChain? _selectedChain = EvmChain.mainnetChains.first;
  bool _showTestnets = false;
  bool _isLoading = false;
  SignResult? _result;
  String? _error;

  @override
  void initState() {
    super.initState();
    ref.read(authStateProvider.notifier).refreshSession();
  }

  @override
  void dispose() {
    _messageHashController.dispose();
    _descriptionController.dispose();
    super.dispose();
  }

  Future<void> _sign() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() {
      _isLoading = true;
      _error = null;
      _result = null;
    });

    try {
      final client = ref.read(daemonClientProvider);
      final result = await client.signEvm(
        messageHash: _messageHashController.text.trim(),
        chainId: _selectedChain!.chainId,
        description: _descriptionController.text.trim(),
      );

      // Save to transaction history
      final cache = ref.read(localCacheProvider);
      await cache.addTransaction(TransactionRecord(
        id: DateTime.now().millisecondsSinceEpoch.toString(),
        signature: result.signature,
        presigIndex: result.presigIndex,
        chainId: _selectedChain!.chainId,
        chainName: _selectedChain!.name,
        description: _descriptionController.text.trim(),
        timestamp: DateTime.now(),
        scheme: 'ecdsa',
      ));

      setState(() {
        _result = result;
        _isLoading = false;
      });

      // Refresh disk status
      ref.invalidate(combinedDiskStatusProvider);
    } catch (e) {
      setState(() {
        _error = e.toString();
        _isLoading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final connectionStatus = ref.watch(daemonConnectionStatusProvider);
    final isOffline = connectionStatus != DaemonConnectionStatus.connected;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Sign EVM Transaction'),
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16),
        child: Form(
          key: _formKey,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // Offline warning
              if (isOffline)
                Container(
                  margin: const EdgeInsets.only(bottom: 16),
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: AppTheme.warningColor.withAlpha(25),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: AppTheme.warningColor.withAlpha(75)),
                  ),
                  child: Row(
                    children: [
                      const Icon(Icons.cloud_off, color: AppTheme.warningColor),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Text(
                          'You are offline. Signing requires a connection to the daemon.',
                          style: TextStyle(color: AppTheme.warningColor),
                        ),
                      ),
                    ],
                  ),
                ),

              // Chain selector
              Text('Network', style: theme.textTheme.titleMedium),
              const SizedBox(height: 8),
              _buildChainSelector(),
              const SizedBox(height: 8),
              Row(
                children: [
                  Checkbox(
                    value: _showTestnets,
                    onChanged: (v) => setState(() => _showTestnets = v ?? false),
                  ),
                  const Text('Show testnets'),
                ],
              ),
              const SizedBox(height: 16),

              // Message hash input
              Text('Message Hash', style: theme.textTheme.titleMedium),
              const SizedBox(height: 8),
              TextFormField(
                controller: _messageHashController,
                decoration: const InputDecoration(
                  hintText: '0x...',
                  helperText: '32-byte hex hash (64 characters with 0x prefix)',
                ),
                style: const TextStyle(fontFamily: 'monospace'),
                maxLines: 2,
                validator: (value) {
                  if (value == null || value.isEmpty) {
                    return 'Message hash is required';
                  }
                  final cleaned = value.trim().toLowerCase();
                  if (!cleaned.startsWith('0x')) {
                    return 'Must start with 0x';
                  }
                  if (cleaned.length != 66) {
                    return 'Must be 64 hex characters (32 bytes)';
                  }
                  if (!RegExp(r'^0x[a-f0-9]{64}$').hasMatch(cleaned)) {
                    return 'Invalid hex format';
                  }
                  return null;
                },
              ),
              const SizedBox(height: 16),

              // Description
              Text('Description', style: theme.textTheme.titleMedium),
              const SizedBox(height: 8),
              TextFormField(
                controller: _descriptionController,
                decoration: const InputDecoration(
                  hintText: 'What is this transaction for?',
                  helperText: 'This will be recorded in the audit log',
                ),
                maxLength: 256,
                validator: (value) {
                  if (value == null || value.trim().isEmpty) {
                    return 'Description is required for audit purposes';
                  }
                  return null;
                },
              ),
              const SizedBox(height: 24),

              // Sign button
              ElevatedButton(
                onPressed: isOffline || _isLoading ? null : _sign,
                child: _isLoading
                    ? const SizedBox(
                        height: 20,
                        width: 20,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Text('Sign Transaction'),
              ),

              // Error display
              if (_error != null) ...[
                const SizedBox(height: 16),
                Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.error.withAlpha(25),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Row(
                    children: [
                      Icon(Icons.error_outline, color: theme.colorScheme.error),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          _error!,
                          style: TextStyle(color: theme.colorScheme.error),
                        ),
                      ),
                    ],
                  ),
                ),
              ],

              // Result display
              if (_result != null) ...[
                const SizedBox(height: 24),
                _buildResultCard(theme),
              ],
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildChainSelector() {
    final chains = _showTestnets
        ? [...EvmChain.mainnetChains, ...EvmChain.testnetChains]
        : EvmChain.mainnetChains;

    return DropdownButtonFormField<EvmChain>(
      value: _selectedChain,
      decoration: const InputDecoration(
        prefixIcon: Icon(Icons.language),
      ),
      items: chains.map((chain) {
        return DropdownMenuItem(
          value: chain,
          child: Row(
            children: [
              Text(chain.name),
              if (chain.isTestnet)
                Container(
                  margin: const EdgeInsets.only(left: 8),
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                    color: Colors.orange.withAlpha(50),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: const Text(
                    'Testnet',
                    style: TextStyle(fontSize: 10, color: Colors.orange),
                  ),
                ),
            ],
          ),
        );
      }).toList(),
      onChanged: (chain) => setState(() => _selectedChain = chain),
    );
  }

  Widget _buildResultCard(ThemeData theme) {
    return Card(
      color: AppTheme.successColor.withAlpha(25),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.check_circle, color: AppTheme.successColor),
                const SizedBox(width: 8),
                Text(
                  'Signature Created',
                  style: theme.textTheme.titleMedium?.copyWith(
                    color: AppTheme.successColor,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            _buildResultRow('Presig Index', '#${_result!.presigIndex}'),
            const Divider(height: 24),
            Text('Signature', style: theme.textTheme.labelMedium),
            const SizedBox(height: 4),
            _buildCopyableText(_result!.signature),
            const SizedBox(height: 12),
            if (_result!.v != null) ...[
              Row(
                children: [
                  Expanded(child: _buildResultRow('v', _result!.v.toString())),
                ],
              ),
              const SizedBox(height: 8),
            ],
            if (_result!.r != null) ...[
              Text('r', style: theme.textTheme.labelMedium),
              const SizedBox(height: 4),
              _buildCopyableText(_result!.r!),
              const SizedBox(height: 8),
            ],
            if (_result!.s != null) ...[
              Text('s', style: theme.textTheme.labelMedium),
              const SizedBox(height: 4),
              _buildCopyableText(_result!.s!),
            ],
            const Divider(height: 24),
            Text('Proof Hash', style: theme.textTheme.labelMedium),
            const SizedBox(height: 4),
            _buildCopyableText(_result!.proofHash),
          ],
        ),
      ),
    );
  }

  Widget _buildResultRow(String label, String value) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(label, style: const TextStyle(color: Colors.grey)),
        Text(value, style: const TextStyle(fontWeight: FontWeight.w500)),
      ],
    );
  }

  Widget _buildCopyableText(String text) {
    return Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: Colors.black.withAlpha(12),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Row(
        children: [
          Expanded(
            child: Text(
              text,
              style: const TextStyle(
                fontFamily: 'monospace',
                fontSize: 12,
              ),
              maxLines: 3,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          IconButton(
            icon: const Icon(Icons.copy, size: 18),
            onPressed: () {
              Clipboard.setData(ClipboardData(text: text));
              ScaffoldMessenger.of(context).showSnackBar(
                const SnackBar(content: Text('Copied to clipboard')),
              );
            },
            tooltip: 'Copy',
          ),
        ],
      ),
    );
  }
}
