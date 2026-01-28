import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:pinput/pinput.dart';
import 'package:sigil_mobile/core/auth/auth_provider.dart';
import 'package:sigil_mobile/core/storage/secure_storage_service.dart';
import 'package:sigil_mobile/shared/widgets/secure_input_wrapper.dart';

/// PIN entry screen for returning users
class PinEntryScreen extends ConsumerStatefulWidget {
  const PinEntryScreen({super.key});

  @override
  ConsumerState<PinEntryScreen> createState() => _PinEntryScreenState();
}

class _PinEntryScreenState extends ConsumerState<PinEntryScreen>
    with SingleTickerProviderStateMixin {
  final _pinController = TextEditingController();
  final _focusNode = FocusNode();
  late AnimationController _shakeController;

  String? _error;
  bool _isLoading = false;
  bool _obscurePin = true;

  @override
  void initState() {
    super.initState();
    _shakeController = AnimationController(
      duration: const Duration(milliseconds: 500),
      vsync: this,
    );
    _tryBiometric();
  }

  @override
  void dispose() {
    _pinController.dispose();
    _focusNode.dispose();
    _shakeController.dispose();
    super.dispose();
  }

  Future<void> _tryBiometric() async {
    final authState = ref.read(authStateProvider);
    if (authState.isBiometricAvailable && authState.isBiometricEnabled) {
      final success =
          await ref.read(authStateProvider.notifier).authenticateWithBiometrics();
      if (success && mounted) {
        context.go('/');
      }
    }
  }

  Future<void> _onPinSubmit(String pin) async {
    if (pin.length < 6) return;

    setState(() {
      _isLoading = true;
      _error = null;
    });

    try {
      final success = await ref.read(authStateProvider.notifier).verifyPin(pin);
      if (success) {
        if (mounted) {
          context.go('/');
        }
      } else {
        _handleWrongPin();
      }
    } on PinLockoutException catch (e) {
      setState(() {
        _error = e.toString();
        _isLoading = false;
      });
      _pinController.clear();
    } catch (e) {
      setState(() {
        _error = 'Verification failed: $e';
        _isLoading = false;
      });
      _pinController.clear();
    }
  }

  void _handleWrongPin() {
    final authState = ref.read(authStateProvider);
    setState(() {
      _error = 'Incorrect PIN. ${authState.remainingAttempts} attempts remaining.';
      _isLoading = false;
    });
    _pinController.clear();
    _focusNode.requestFocus();

    // Shake animation
    _shakeController.forward(from: 0);
    HapticFeedback.heavyImpact();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final authState = ref.watch(authStateProvider);

    final defaultPinTheme = PinTheme(
      width: 50,
      height: 56,
      textStyle: theme.textTheme.headlineMedium?.copyWith(
        fontWeight: FontWeight.bold,
      ),
      decoration: BoxDecoration(
        border: Border.all(color: Colors.grey.shade300),
        borderRadius: BorderRadius.circular(8),
      ),
    );

    final focusedPinTheme = defaultPinTheme.copyWith(
      decoration: BoxDecoration(
        border: Border.all(color: theme.colorScheme.primary, width: 2),
        borderRadius: BorderRadius.circular(8),
      ),
    );

    final errorPinTheme = defaultPinTheme.copyWith(
      decoration: BoxDecoration(
        border: Border.all(color: theme.colorScheme.error, width: 2),
        borderRadius: BorderRadius.circular(8),
      ),
    );

    // Check for lockout
    final isLockedOut = authState.lockoutRemaining != null;

    return Scaffold(
      body: SecureInputWrapper(
        child: SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const Spacer(flex: 1),
                // Logo
                Icon(
                  Icons.shield_outlined,
                  size: 80,
                  color: theme.colorScheme.primary,
                ),
                const SizedBox(height: 24),
                Text(
                  'Welcome Back',
                  style: theme.textTheme.headlineLarge,
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 8),
                Text(
                  'Enter your PIN to continue',
                  style: theme.textTheme.bodyLarge?.copyWith(
                    color: Colors.grey,
                  ),
                  textAlign: TextAlign.center,
                ),
                const Spacer(flex: 1),
                // Lockout message
                if (isLockedOut)
                  _buildLockoutMessage(theme, authState.lockoutRemaining!)
                else ...[
                  // PIN input with shake animation
                  Center(
                    child: AnimatedBuilder(
                      animation: _shakeController,
                      builder: (context, child) {
                        final offset = _shakeAnimation.evaluate(_shakeController);
                        return Transform.translate(
                          offset: Offset(offset, 0),
                          child: child,
                        );
                      },
                      child: _isLoading
                          ? const CircularProgressIndicator()
                          : Pinput(
                              controller: _pinController,
                              focusNode: _focusNode,
                              length: 6,
                              obscureText: _obscurePin,
                              obscuringCharacter: '●',
                              defaultPinTheme: defaultPinTheme,
                              focusedPinTheme: focusedPinTheme,
                              errorPinTheme: _error != null ? errorPinTheme : null,
                              keyboardType: TextInputType.number,
                              autofocus: true,
                              onCompleted: _onPinSubmit,
                              hapticFeedbackType: HapticFeedbackType.lightImpact,
                            ),
                    ),
                  ),
                  const SizedBox(height: 16),
                  // Toggle visibility
                  TextButton.icon(
                    onPressed: () {
                      setState(() {
                        _obscurePin = !_obscurePin;
                      });
                    },
                    icon: Icon(
                      _obscurePin ? Icons.visibility : Icons.visibility_off,
                    ),
                    label: Text(_obscurePin ? 'Show PIN' : 'Hide PIN'),
                  ),
                ],
                const SizedBox(height: 16),
                // Error message
                if (_error != null)
                  Container(
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: theme.colorScheme.error.withAlpha(25),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Row(
                      children: [
                        Icon(
                          Icons.error_outline,
                          color: theme.colorScheme.error,
                        ),
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
                const Spacer(flex: 1),
                // Biometric button
                if (authState.isBiometricAvailable &&
                    authState.isBiometricEnabled &&
                    !isLockedOut)
                  OutlinedButton.icon(
                    onPressed: _tryBiometric,
                    icon: const Icon(Icons.fingerprint),
                    label: const Text('Use Biometrics'),
                  ),
                const SizedBox(height: 24),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildLockoutMessage(ThemeData theme, Duration remaining) {
    return Container(
      padding: const EdgeInsets.all(24),
      decoration: BoxDecoration(
        color: theme.colorScheme.error.withAlpha(25),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Column(
        children: [
          Icon(
            Icons.lock_clock,
            size: 48,
            color: theme.colorScheme.error,
          ),
          const SizedBox(height: 16),
          Text(
            'Too Many Attempts',
            style: theme.textTheme.titleLarge?.copyWith(
              color: theme.colorScheme.error,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'Please wait ${remaining.inMinutes}:${(remaining.inSeconds % 60).toString().padLeft(2, '0')} before trying again.',
            style: theme.textTheme.bodyMedium,
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }

  // Shake animation
  static final _shakeAnimation = TweenSequence<double>([
    TweenSequenceItem(tween: Tween(begin: 0, end: -10), weight: 1),
    TweenSequenceItem(tween: Tween(begin: -10, end: 10), weight: 2),
    TweenSequenceItem(tween: Tween(begin: 10, end: -10), weight: 2),
    TweenSequenceItem(tween: Tween(begin: -10, end: 10), weight: 2),
    TweenSequenceItem(tween: Tween(begin: 10, end: 0), weight: 1),
  ]);
}
