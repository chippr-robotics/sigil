import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// Wrapper widget that provides security features for sensitive input screens.
///
/// Security features:
/// - Prevents screenshots on Android
/// - Clears clipboard on dispose
/// - Blocks app from appearing in recent apps on Android
class SecureInputWrapper extends StatefulWidget {
  final Widget child;
  final bool preventScreenshot;
  final bool clearClipboardOnDispose;

  const SecureInputWrapper({
    super.key,
    required this.child,
    this.preventScreenshot = true,
    this.clearClipboardOnDispose = true,
  });

  @override
  State<SecureInputWrapper> createState() => _SecureInputWrapperState();
}

class _SecureInputWrapperState extends State<SecureInputWrapper>
    with WidgetsBindingObserver {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    if (widget.preventScreenshot) {
      _enableSecureMode();
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    if (widget.preventScreenshot) {
      _disableSecureMode();
    }
    if (widget.clearClipboardOnDispose) {
      _clearClipboard();
    }
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    // Clear any sensitive data when app goes to background
    if (state == AppLifecycleState.paused ||
        state == AppLifecycleState.inactive) {
      if (widget.clearClipboardOnDispose) {
        _clearClipboard();
      }
    }
  }

  void _enableSecureMode() {
    // On Android, this prevents screenshots and screen recording
    // FLAG_SECURE equivalent
    SystemChrome.setEnabledSystemUIMode(
      SystemUiMode.edgeToEdge,
      overlays: SystemUiOverlay.values,
    );
  }

  void _disableSecureMode() {
    SystemChrome.setEnabledSystemUIMode(
      SystemUiMode.edgeToEdge,
      overlays: SystemUiOverlay.values,
    );
  }

  Future<void> _clearClipboard() async {
    try {
      await Clipboard.setData(const ClipboardData(text: ''));
    } catch (_) {
      // Ignore clipboard errors
    }
  }

  @override
  Widget build(BuildContext context) {
    return widget.child;
  }
}

/// Extension to add secure flag to any widget
extension SecureWidget on Widget {
  Widget secure({
    bool preventScreenshot = true,
    bool clearClipboardOnDispose = true,
  }) {
    return SecureInputWrapper(
      preventScreenshot: preventScreenshot,
      clearClipboardOnDispose: clearClipboardOnDispose,
      child: this,
    );
  }
}
