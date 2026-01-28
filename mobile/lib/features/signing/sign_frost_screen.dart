import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:sigil_mobile/core/api/daemon_client.dart';
import 'package:sigil_mobile/core/auth/auth_provider.dart';
import 'package:sigil_mobile/core/models/models.dart';
import 'package:sigil_mobile/core/storage/local_cache_service.dart';
import 'package:sigil_mobile/features/dashboard/dashboard_screen.dart';
import 'package:sigil_mobile/shared/theme/app_theme.dart';

/// FROST signing screen for Bitcoin, Solana, Cosmos, etc.
class SignFrostScreen extends ConsumerStatefulWidget {
  const SignFrostScreen({super.key});

  @override
  ConsumerState<SignFrostScreen> createState() => _SignFrostScreenState();
}

class _SignFrostScreenState extends ConsumerState<SignFrostScreen> {
  final _formKey = GlobalKey<FormState>();
  final _messageHashController = TextEditingController();
  final _descriptionController = TextEditingController();

  FrostScheme _selectedScheme = FrostScheme.taproot;
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
      final result = await client.signFrost(
        scheme: _selectedScheme.value,
        messageHash: _messageHashController.text.trim(),
        description: _descriptionController.text.trim(),
      );

      // Save to transaction history
      final cache = ref.read(localCacheProvider);
      await cache.addTransaction(TransactionRecord(
        id: DateTime.now().millisecondsSinceEpoch.toString(),
        signature: result.signature,
        presigIndex: result.presigIndex,
        description: _descriptionController.text.trim(),
        timestamp: DateTime.now(),
        scheme: _selectedScheme.value,
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
        title: const Text('Sign with FROST'),
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

              // Scheme info card
              _buildSchemeInfoCard(theme),
              const SizedBox(height: 24),

              // Scheme selector
              Text('Signature Scheme', style: theme.textTheme.titleMedium),
              const SizedBox(height: 8),
              ...FrostScheme.values.map((scheme) => _buildSchemeOption(scheme)),
              const SizedBox(height: 16),

              // Message hash input
              Text('Message Hash', style: theme.textTheme.titleMedium),
              const SizedBox(height: 8),
              TextFormField(
                controller: _messageHashController,
                decoration: InputDecoration(
                  hintText: '0x...',
                  helperText: _getHashHelperText(),
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
                  // FROST can have variable length messages
                  if (cleaned.length < 4) {
                    return 'Hash too short';
                  }
                  if (!RegExp(r'^0x[a-f0-9]+$').hasMatch(cleaned)) {
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
                    : const Text('Sign Message'),
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

  Widget _buildSchemeInfoCard(ThemeData theme) {
    final (icon, chains) = switch (_selectedScheme) {
      FrostScheme.taproot => (
          Icons.currency_bitcoin,
          'Bitcoin (Taproot/BIP-340)',
        ),
      FrostScheme.ed25519 => (
          Icons.blur_circular,
          'Solana, Cosmos, Near, Polkadot, Cardano',
        ),
      FrostScheme.ristretto255 => (
          Icons.security,
          'Zcash (shielded transactions)',
        ),
    };

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            Container(
              padding: const EdgeInsets.all(12),
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
                  Text(
                    _selectedScheme.displayName,
                    style: theme.textTheme.titleMedium,
                  ),
                  const SizedBox(height: 4),
                  Text(
                    chains,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: Colors.grey,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildSchemeOption(FrostScheme scheme) {
    final isSelected = _selectedScheme == scheme;
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: InkWell(
        onTap: () => setState(() => _selectedScheme = scheme),
        borderRadius: BorderRadius.circular(8),
        child: Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            border: Border.all(
              color: isSelected
                  ? theme.colorScheme.primary
                  : Colors.grey.shade300,
              width: isSelected ? 2 : 1,
            ),
            borderRadius: BorderRadius.circular(8),
            color: isSelected
                ? theme.colorScheme.primary.withAlpha(12)
                : null,
          ),
          child: Row(
            children: [
              Radio<FrostScheme>(
                value: scheme,
                groupValue: _selectedScheme,
                onChanged: (v) => setState(() => _selectedScheme = v!),
              ),
              const SizedBox(width: 8),
              Text(
                scheme.displayName,
                style: theme.textTheme.bodyLarge?.copyWith(
                  fontWeight: isSelected ? FontWeight.w600 : null,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  String _getHashHelperText() {
    return switch (_selectedScheme) {
      FrostScheme.taproot => '32-byte hash for BIP-340 signing',
      FrostScheme.ed25519 => 'Message hash for Ed25519 signing',
      FrostScheme.ristretto255 => 'Message hash for Ristretto255 signing',
    };
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
            _buildResultRow('Scheme', _selectedScheme.displayName),
            _buildResultRow('Presig Index', '#${_result!.presigIndex}'),
            const Divider(height: 24),
            Text('Signature (64 bytes)', style: theme.textTheme.labelMedium),
            const SizedBox(height: 4),
            _buildCopyableText(_result!.signature),
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
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: const TextStyle(color: Colors.grey)),
          Text(value, style: const TextStyle(fontWeight: FontWeight.w500)),
        ],
      ),
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
