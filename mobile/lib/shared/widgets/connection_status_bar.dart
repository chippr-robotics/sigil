import 'package:flutter/material.dart';
import 'package:sigil_mobile/core/models/models.dart';
import 'package:sigil_mobile/shared/theme/app_theme.dart';

/// Connection status bar widget
class ConnectionStatusBar extends StatelessWidget {
  final DaemonConnectionStatus status;

  const ConnectionStatusBar({
    super.key,
    required this.status,
  });

  @override
  Widget build(BuildContext context) {
    final (color, icon, text) = switch (status) {
      DaemonConnectionStatus.connected => (
          AppTheme.successColor,
          Icons.cloud_done,
          'Connected to daemon',
        ),
      DaemonConnectionStatus.connecting => (
          AppTheme.warningColor,
          Icons.cloud_sync,
          'Connecting...',
        ),
      DaemonConnectionStatus.disconnected => (
          Colors.grey,
          Icons.cloud_off,
          'Offline mode',
        ),
      DaemonConnectionStatus.error => (
          AppTheme.errorColor,
          Icons.cloud_off,
          'Connection error',
        ),
    };

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: color.withAlpha(25),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: color.withAlpha(75)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 18, color: color),
          const SizedBox(width: 8),
          Text(
            text,
            style: TextStyle(
              color: color,
              fontWeight: FontWeight.w500,
              fontSize: 13,
            ),
          ),
        ],
      ),
    );
  }
}
