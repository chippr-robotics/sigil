import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:pinput/pinput.dart';
import 'package:sigil_mobile/core/auth/auth_provider.dart';
import 'package:sigil_mobile/shared/theme/app_theme.dart';
import 'package:sigil_mobile/shared/widgets/secure_input_wrapper.dart';

/// PIN setup screen for first-time users
class PinSetupScreen extends ConsumerStatefulWidget {
  const PinSetupScreen({super.key});

  @override
  ConsumerState<PinSetupScreen> createState() => _PinSetupScreenState();
}

class _PinSetupScreenState extends ConsumerState<PinSetupScreen> {
  final _pinController = TextEditingController();
  final _confirmPinController = TextEditingController();
  final _focusNode = FocusNode();
  final _confirmFocusNode = FocusNode();

  bool _isConfirmStep = false;
  String _firstPin = '';
  String? _error;
  bool _isLoading = false;
  bool _obscurePin = true;

  @override
  void dispose() {
    _pinController.dispose();
    _confirmPinController.dispose();
    _focusNode.dispose();
    _confirmFocusNode.dispose();
    super.dispose();
  }

  Future<void> _onPinSubmit(String pin) async {
    if (pin.length < 6) return;

    if (!_isConfirmStep) {
      // First entry
      setState(() {
        _firstPin = pin;
        _isConfirmStep = true;
        _error = null;
      });
      _pinController.clear();
      _confirmFocusNode.requestFocus();
    } else {
      // Confirm entry
      if (pin != _firstPin) {
        setState(() {
          _error = 'PINs do not match. Please try again.';
          _isConfirmStep = false;
          _firstPin = '';
        });
        _confirmPinController.clear();
        _focusNode.requestFocus();
        HapticFeedback.heavyImpact();
        return;
      }

      // PINs match - setup
      setState(() {
        _isLoading = true;
        _error = null;
      });

      try {
        await ref.read(authStateProvider.notifier).setupPin(pin);
        if (mounted) {
          context.go('/');
        }
      } catch (e) {
        setState(() {
          _error = 'Failed to setup PIN: $e';
          _isLoading = false;
          _isConfirmStep = false;
          _firstPin = '';
        });
        _pinController.clear();
        _confirmPinController.clear();
      }
    }
  }

  void _reset() {
    setState(() {
      _isConfirmStep = false;
      _firstPin = '';
      _error = null;
    });
    _pinController.clear();
    _confirmPinController.clear();
    _focusNode.requestFocus();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

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

    return Scaffold(
      body: SecureInputWrapper(
        child: SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const Spacer(flex: 1),
                // Logo/Title
                Icon(
                  Icons.shield_outlined,
                  size: 80,
                  color: theme.colorScheme.primary,
                ),
                const SizedBox(height: 24),
                Text(
                  'Sigil',
                  style: theme.textTheme.headlineLarge,
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 8),
                Text(
                  'Secure MPC Signing',
                  style: theme.textTheme.bodyLarge?.copyWith(
                    color: Colors.grey,
                  ),
                  textAlign: TextAlign.center,
                ),
                const Spacer(flex: 1),
                // Instructions
                Text(
                  _isConfirmStep ? 'Confirm your PIN' : 'Create a 6-digit PIN',
                  style: theme.textTheme.titleLarge,
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 8),
                Text(
                  _isConfirmStep
                      ? 'Enter the same PIN again to confirm'
                      : 'This PIN will protect your signing operations',
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: Colors.grey,
                  ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 32),
                // PIN input
                Center(
                  child: _isLoading
                      ? const CircularProgressIndicator()
                      : Pinput(
                          controller: _isConfirmStep
                              ? _confirmPinController
                              : _pinController,
                          focusNode: _isConfirmStep ? _confirmFocusNode : _focusNode,
                          length: 6,
                          obscureText: _obscurePin,
                          obscuringCharacter: '●',
                          defaultPinTheme: defaultPinTheme,
                          focusedPinTheme: focusedPinTheme,
                          errorPinTheme: errorPinTheme,
                          pinputAutovalidateMode: PinputAutovalidateMode.disabled,
                          keyboardType: TextInputType.number,
                          autofocus: true,
                          onCompleted: _onPinSubmit,
                          hapticFeedbackType: HapticFeedbackType.lightImpact,
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
                // Reset button
                if (_isConfirmStep)
                  OutlinedButton(
                    onPressed: _reset,
                    child: const Text('Start Over'),
                  ),
                const SizedBox(height: 16),
                // Security note
                Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Icon(
                      Icons.lock_outline,
                      size: 16,
                      color: Colors.grey.shade600,
                    ),
                    const SizedBox(width: 8),
                    Text(
                      'Your PIN is securely stored on device',
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: Colors.grey.shade600,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 24),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
