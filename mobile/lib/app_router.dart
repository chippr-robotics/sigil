import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:sigil_mobile/core/auth/auth_provider.dart';
import 'package:sigil_mobile/features/auth/pin_setup_screen.dart';
import 'package:sigil_mobile/features/auth/pin_entry_screen.dart';
import 'package:sigil_mobile/features/dashboard/dashboard_screen.dart';
import 'package:sigil_mobile/features/signing/sign_evm_screen.dart';
import 'package:sigil_mobile/features/signing/sign_frost_screen.dart';
import 'package:sigil_mobile/features/addresses/addresses_screen.dart';
import 'package:sigil_mobile/features/settings/settings_screen.dart';
import 'package:sigil_mobile/features/settings/daemon_config_screen.dart';

final appRouterProvider = Provider<GoRouter>((ref) {
  final authState = ref.watch(authStateProvider);

  return GoRouter(
    initialLocation: '/',
    redirect: (context, state) {
      final isAuthenticated = authState.isAuthenticated;
      final isPinSetup = authState.isPinSetup;
      final isGoingToAuth = state.matchedLocation.startsWith('/auth');

      // If PIN is not setup, redirect to PIN setup
      if (!isPinSetup && state.matchedLocation != '/auth/setup') {
        return '/auth/setup';
      }

      // If not authenticated and not going to auth, redirect to PIN entry
      if (!isAuthenticated && !isGoingToAuth) {
        return '/auth/pin';
      }

      // If authenticated and going to auth, redirect to dashboard
      if (isAuthenticated && isGoingToAuth) {
        return '/';
      }

      return null;
    },
    routes: [
      GoRoute(
        path: '/',
        name: 'dashboard',
        builder: (context, state) => const DashboardScreen(),
      ),
      GoRoute(
        path: '/auth/setup',
        name: 'pin-setup',
        builder: (context, state) => const PinSetupScreen(),
      ),
      GoRoute(
        path: '/auth/pin',
        name: 'pin-entry',
        builder: (context, state) => const PinEntryScreen(),
      ),
      GoRoute(
        path: '/sign/evm',
        name: 'sign-evm',
        builder: (context, state) => const SignEvmScreen(),
      ),
      GoRoute(
        path: '/sign/frost',
        name: 'sign-frost',
        builder: (context, state) => const SignFrostScreen(),
      ),
      GoRoute(
        path: '/addresses',
        name: 'addresses',
        builder: (context, state) => const AddressesScreen(),
      ),
      GoRoute(
        path: '/settings',
        name: 'settings',
        builder: (context, state) => const SettingsScreen(),
      ),
      GoRoute(
        path: '/settings/daemon',
        name: 'daemon-config',
        builder: (context, state) => const DaemonConfigScreen(),
      ),
    ],
    errorBuilder: (context, state) => Scaffold(
      body: Center(
        child: Text('Page not found: ${state.uri}'),
      ),
    ),
  );
});
