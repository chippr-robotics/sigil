import 'dart:async';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:local_auth/local_auth.dart';
import 'package:sigil_mobile/core/storage/secure_storage_service.dart';

/// Authentication state
class AuthState {
  final bool isPinSetup;
  final bool isAuthenticated;
  final bool isBiometricAvailable;
  final bool isBiometricEnabled;
  final int remainingAttempts;
  final Duration? lockoutRemaining;

  const AuthState({
    this.isPinSetup = false,
    this.isAuthenticated = false,
    this.isBiometricAvailable = false,
    this.isBiometricEnabled = false,
    this.remainingAttempts = SecureStorageService.maxFailedAttempts,
    this.lockoutRemaining,
  });

  AuthState copyWith({
    bool? isPinSetup,
    bool? isAuthenticated,
    bool? isBiometricAvailable,
    bool? isBiometricEnabled,
    int? remainingAttempts,
    Duration? lockoutRemaining,
  }) {
    return AuthState(
      isPinSetup: isPinSetup ?? this.isPinSetup,
      isAuthenticated: isAuthenticated ?? this.isAuthenticated,
      isBiometricAvailable: isBiometricAvailable ?? this.isBiometricAvailable,
      isBiometricEnabled: isBiometricEnabled ?? this.isBiometricEnabled,
      remainingAttempts: remainingAttempts ?? this.remainingAttempts,
      lockoutRemaining: lockoutRemaining,
    );
  }
}

/// Secure storage provider
final secureStorageProvider = Provider<SecureStorageService>((ref) {
  return SecureStorageService();
});

/// Local auth (biometric) provider
final localAuthProvider = Provider<LocalAuthentication>((ref) {
  return LocalAuthentication();
});

/// Authentication state notifier
class AuthStateNotifier extends StateNotifier<AuthState> {
  final SecureStorageService _storage;
  final LocalAuthentication _localAuth;
  Timer? _sessionTimer;
  Timer? _lockoutTimer;

  AuthStateNotifier(this._storage, this._localAuth) : super(const AuthState()) {
    _initialize();
  }

  Future<void> _initialize() async {
    final isPinSetup = await _storage.isPinSetup();
    final isSessionValid = await _storage.isSessionValid();
    final isBiometricAvailable = await _localAuth.canCheckBiometrics;
    final isBiometricEnabled = await _storage.isBiometricEnabled();
    final remainingAttempts = await _storage.getRemainingAttempts();
    final lockoutRemaining = await _storage.getLockoutRemaining();

    state = state.copyWith(
      isPinSetup: isPinSetup,
      isAuthenticated: isSessionValid,
      isBiometricAvailable: isBiometricAvailable,
      isBiometricEnabled: isBiometricEnabled,
      remainingAttempts: remainingAttempts,
      lockoutRemaining: lockoutRemaining,
    );

    if (isSessionValid) {
      _startSessionTimer();
    }

    if (lockoutRemaining != null) {
      _startLockoutTimer(lockoutRemaining);
    }
  }

  void _startSessionTimer() {
    _sessionTimer?.cancel();
    _sessionTimer = Timer.periodic(
      const Duration(minutes: 1),
      (_) => _checkSessionTimeout(),
    );
  }

  void _startLockoutTimer(Duration remaining) {
    _lockoutTimer?.cancel();
    _lockoutTimer = Timer(remaining, () {
      state = state.copyWith(
        lockoutRemaining: null,
        remainingAttempts: SecureStorageService.maxFailedAttempts,
      );
    });
  }

  Future<void> _checkSessionTimeout() async {
    final isValid = await _storage.isSessionValid();
    if (!isValid && state.isAuthenticated) {
      state = state.copyWith(isAuthenticated: false);
      _sessionTimer?.cancel();
    }
  }

  /// Setup new PIN
  Future<void> setupPin(String pin) async {
    await _storage.setupPin(pin);
    state = state.copyWith(
      isPinSetup: true,
      isAuthenticated: true,
      remainingAttempts: SecureStorageService.maxFailedAttempts,
    );
    _startSessionTimer();
  }

  /// Verify PIN and authenticate
  Future<bool> verifyPin(String pin) async {
    try {
      final success = await _storage.verifyPin(pin);
      if (success) {
        state = state.copyWith(
          isAuthenticated: true,
          remainingAttempts: SecureStorageService.maxFailedAttempts,
          lockoutRemaining: null,
        );
        _startSessionTimer();
        return true;
      } else {
        final remaining = await _storage.getRemainingAttempts();
        state = state.copyWith(remainingAttempts: remaining);
        return false;
      }
    } on PinLockoutException catch (e) {
      state = state.copyWith(
        remainingAttempts: 0,
        lockoutRemaining: e.remainingTime,
      );
      _startLockoutTimer(e.remainingTime);
      rethrow;
    }
  }

  /// Authenticate with biometrics
  Future<bool> authenticateWithBiometrics() async {
    if (!state.isBiometricAvailable || !state.isBiometricEnabled) {
      return false;
    }

    try {
      final success = await _localAuth.authenticate(
        localizedReason: 'Authenticate to access Sigil',
        options: const AuthenticationOptions(
          stickyAuth: true,
          biometricOnly: true,
        ),
      );

      if (success) {
        await _storage.refreshSession();
        state = state.copyWith(isAuthenticated: true);
        _startSessionTimer();
      }

      return success;
    } catch (e) {
      return false;
    }
  }

  /// Enable/disable biometric authentication
  Future<void> setBiometricEnabled(bool enabled) async {
    await _storage.setBiometricEnabled(enabled);
    state = state.copyWith(isBiometricEnabled: enabled);
  }

  /// Change PIN
  Future<void> changePin(String currentPin, String newPin) async {
    await _storage.changePin(currentPin, newPin);
  }

  /// Refresh session (call on user activity)
  Future<void> refreshSession() async {
    if (state.isAuthenticated) {
      await _storage.refreshSession();
    }
  }

  /// Logout
  Future<void> logout() async {
    await _storage.clearSession();
    _sessionTimer?.cancel();
    state = state.copyWith(isAuthenticated: false);
  }

  /// Wipe all data and reset
  Future<void> wipeAllData() async {
    await _storage.wipeAllData();
    _sessionTimer?.cancel();
    _lockoutTimer?.cancel();
    state = const AuthState();
  }

  @override
  void dispose() {
    _sessionTimer?.cancel();
    _lockoutTimer?.cancel();
    super.dispose();
  }
}

/// Authentication state provider
final authStateProvider = StateNotifierProvider<AuthStateNotifier, AuthState>((ref) {
  final storage = ref.watch(secureStorageProvider);
  final localAuth = ref.watch(localAuthProvider);
  return AuthStateNotifier(storage, localAuth);
});
