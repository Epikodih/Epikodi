import 'package:flutter/material.dart';

import 'connected/connected_screen.dart';
import 'deps.dart';
import 'l10n/app_localizations.dart';
import 'locale.dart';
import 'pairing/pairing_screen.dart';
import 'storage/remote_store.dart';
import 'theme.dart';

class GloubiRemoteApp extends StatelessWidget {
  const GloubiRemoteApp({
    super.key,
    required this.deps,
    this.locale,
    this.initialPairing,
  });

  final RemoteDeps deps;

  /// `null` : langue du téléphone (français par défaut).
  final Locale? locale;

  /// Pour les tests : démarre directement sur la télécommande, déjà
  /// appairée. Dans l'app, toujours `null` : on s'appaire à chaque lancement.
  final SavedPairing? initialPairing;

  @override
  Widget build(BuildContext context) {
    // Le bouton FR / EN change [remoteLocale] : toute l'app suit.
    return ValueListenableBuilder<Locale?>(
      valueListenable: remoteLocale,
      builder: (context, chosen, _) => _app(chosen ?? locale),
    );
  }

  Widget _app(Locale? locale) {
    return MaterialApp(
      title: 'GloubiTV Remote',
      debugShowCheckedModeBanner: false,
      theme: remoteTheme(),
      locale: locale,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      localeResolutionCallback: (device, supported) => supported.firstWhere(
        (l) => l.languageCode == device?.languageCode,
        orElse: () => const Locale('fr'),
      ),
      // Au-dessus du Navigator : tous les écrans y ont accès.
      builder: (context, child) => RemoteDepsScope(
        deps: deps,
        child: child ?? const SizedBox.shrink(),
      ),
      home: initialPairing == null
          ? const StartScreen()
          : ConnectedScreen(pairing: initialPairing!),
    );
  }
}

/// Au lancement : toujours un nouvel appairage. La TV a pu redémarrer
/// depuis (nouvelle session, nouveau code) : un ancien token ne veut plus
/// rien dire. On l'efface et on repart de la connexion au serveur.
/// L'adresse du serveur, elle, est gardée.
class StartScreen extends StatefulWidget {
  const StartScreen({super.key});

  @override
  State<StartScreen> createState() => _StartScreenState();
}

class _StartScreenState extends State<StartScreen> {
  bool _cleared = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_cleared) return;
    _cleared = true;
    RemoteDeps.of(context).store.clear().catchError((_) {});
  }

  @override
  Widget build(BuildContext context) => const PairingScreen();
}
