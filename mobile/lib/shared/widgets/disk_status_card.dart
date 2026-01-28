import 'package:flutter/material.dart';
import 'package:sigil_mobile/core/models/models.dart';
import 'package:sigil_mobile/shared/theme/app_theme.dart';

/// Card displaying the current disk status
class DiskStatusCard extends StatelessWidget {
  final DiskStatus status;
  final bool isFromCache;
  final DateTime? lastSync;

  const DiskStatusCard({
    super.key,
    required this.status,
    this.isFromCache = false,
    this.lastSync,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    if (!status.detected) {
      return _buildNoDiskCard(context);
    }

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header row
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: status.isValid == true
                        ? AppTheme.successColor.withAlpha(25)
                        : AppTheme.errorColor.withAlpha(25),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Icon(
                    status.isValid == true
                        ? Icons.verified
                        : Icons.warning,
                    color: status.isValid == true
                        ? AppTheme.successColor
                        : AppTheme.errorColor,
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Sigil Disk',
                        style: theme.textTheme.titleMedium,
                      ),
                      Text(
                        status.childId ?? 'Unknown',
                        style: theme.textTheme.bodySmall?.copyWith(
                          fontFamily: 'monospace',
                          color: Colors.grey,
                        ),
                      ),
                    ],
                  ),
                ),
                if (isFromCache)
                  Tooltip(
                    message: lastSync != null
                        ? 'Last synced: ${_formatLastSync(lastSync!)}'
                        : 'Cached data',
                    child: Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 4,
                      ),
                      decoration: BoxDecoration(
                        color: AppTheme.warningColor.withAlpha(25),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(
                            Icons.cloud_off,
                            size: 14,
                            color: AppTheme.warningColor,
                          ),
                          const SizedBox(width: 4),
                          Text(
                            'Offline',
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: AppTheme.warningColor,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
              ],
            ),
            const SizedBox(height: 16),
            const Divider(height: 1),
            const SizedBox(height: 16),

            // Status details
            _buildStatusRow(
              context,
              'Scheme',
              _formatScheme(status.scheme),
              Icons.key,
            ),
            const SizedBox(height: 12),
            _buildPresigRow(context),
            const SizedBox(height: 12),
            _buildExpiryRow(context),

            // Warnings
            if (status.isLowPresigs || status.isExpiringSoon) ...[
              const SizedBox(height: 16),
              _buildWarnings(context),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildNoDiskCard(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
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
              'Insert your Sigil disk to begin signing operations',
              style: theme.textTheme.bodyMedium?.copyWith(
                color: Colors.grey,
              ),
              textAlign: TextAlign.center,
            ),
            if (isFromCache && lastSync != null) ...[
              const SizedBox(height: 16),
              Text(
                'Last seen: ${_formatLastSync(lastSync!)}',
                style: theme.textTheme.bodySmall?.copyWith(
                  color: Colors.grey,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildStatusRow(
    BuildContext context,
    String label,
    String value,
    IconData icon,
  ) {
    final theme = Theme.of(context);

    return Row(
      children: [
        Icon(icon, size: 20, color: Colors.grey),
        const SizedBox(width: 8),
        Text(
          label,
          style: theme.textTheme.bodyMedium?.copyWith(
            color: Colors.grey,
          ),
        ),
        const Spacer(),
        Text(
          value,
          style: theme.textTheme.bodyMedium?.copyWith(
            fontWeight: FontWeight.w500,
          ),
        ),
      ],
    );
  }

  Widget _buildPresigRow(BuildContext context) {
    final theme = Theme.of(context);
    final remaining = status.presigsRemaining ?? 0;
    final total = status.presigsTotal ?? 1;
    final percentage = total > 0 ? remaining / total : 0.0;

    Color progressColor;
    if (percentage > 0.5) {
      progressColor = AppTheme.successColor;
    } else if (percentage > 0.2) {
      progressColor = AppTheme.warningColor;
    } else {
      progressColor = AppTheme.errorColor;
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            const Icon(Icons.inventory_2, size: 20, color: Colors.grey),
            const SizedBox(width: 8),
            Text(
              'Presignatures',
              style: theme.textTheme.bodyMedium?.copyWith(
                color: Colors.grey,
              ),
            ),
            const Spacer(),
            Text(
              '$remaining / $total',
              style: theme.textTheme.bodyMedium?.copyWith(
                fontWeight: FontWeight.w500,
              ),
            ),
          ],
        ),
        const SizedBox(height: 8),
        ClipRRect(
          borderRadius: BorderRadius.circular(4),
          child: LinearProgressIndicator(
            value: percentage,
            backgroundColor: Colors.grey.shade200,
            valueColor: AlwaysStoppedAnimation(progressColor),
            minHeight: 8,
          ),
        ),
      ],
    );
  }

  Widget _buildExpiryRow(BuildContext context) {
    final theme = Theme.of(context);
    final days = status.daysUntilExpiry ?? 0;

    Color textColor;
    if (days > 30) {
      textColor = AppTheme.successColor;
    } else if (days > 7) {
      textColor = AppTheme.warningColor;
    } else {
      textColor = AppTheme.errorColor;
    }

    return Row(
      children: [
        const Icon(Icons.schedule, size: 20, color: Colors.grey),
        const SizedBox(width: 8),
        Text(
          'Expires in',
          style: theme.textTheme.bodyMedium?.copyWith(
            color: Colors.grey,
          ),
        ),
        const Spacer(),
        Text(
          days == 1 ? '1 day' : '$days days',
          style: theme.textTheme.bodyMedium?.copyWith(
            fontWeight: FontWeight.w500,
            color: textColor,
          ),
        ),
      ],
    );
  }

  Widget _buildWarnings(BuildContext context) {
    final warnings = <Widget>[];

    if (status.isLowPresigs) {
      warnings.add(_buildWarningChip(
        context,
        'Low presignatures',
        Icons.warning,
      ));
    }

    if (status.isExpiringSoon) {
      warnings.add(_buildWarningChip(
        context,
        'Expiring soon',
        Icons.schedule,
      ));
    }

    return Wrap(
      spacing: 8,
      runSpacing: 8,
      children: warnings,
    );
  }

  Widget _buildWarningChip(
    BuildContext context,
    String label,
    IconData icon,
  ) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      decoration: BoxDecoration(
        color: AppTheme.warningColor.withAlpha(25),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: AppTheme.warningColor.withAlpha(75)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 16, color: AppTheme.warningColor),
          const SizedBox(width: 6),
          Text(
            label,
            style: TextStyle(
              color: AppTheme.warningColor,
              fontWeight: FontWeight.w500,
              fontSize: 12,
            ),
          ),
        ],
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

  String _formatLastSync(DateTime time) {
    final now = DateTime.now();
    final diff = now.difference(time);

    if (diff.inMinutes < 1) return 'Just now';
    if (diff.inHours < 1) return '${diff.inMinutes} minutes ago';
    if (diff.inDays < 1) return '${diff.inHours} hours ago';
    return '${diff.inDays} days ago';
  }
}
