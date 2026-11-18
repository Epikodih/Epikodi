import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:window_manager/window_manager.dart';

import 'config.dart';
import 'l10n/app_localizations.dart';
import 'locale.dart';
import 'pairing/pairing_screen.dart';
import 'window.dart';

class GloubiTvApp extends StatefulWidget {
  const GloubiTvApp({super.key});

  @override
  State<GloubiTvApp> createState() => _GloubiTvAppState();
}

class _GloubiTvAppState extends State<GloubiTvApp> {
  // Réglages d'affichage globaux (texte, QR, animations), lus dans config.dart.
  final _settings = UiSettings(
    textScale: defaultTextScale,
    qrScale: defaultQrScale,
    reducedMotion: defaultReducedMotion,
  );

  @override
  void initState() {
    super.initState();
    // Écoute le clavier directement, quel que soit l'élément qui a le focus.
    HardwareKeyboard.instance.addHandler(_onKey);
  }

  @override
  void dispose() {
    HardwareKeyboard.instance.removeHandler(_onKey);
    _settings.dispose();
    super.dispose();
  }

  /// Raccourcis fenêtre :
  ///   F11     bascule plein écran / fenêtre
  ///   Échap   quitte le plein écran (si escapeLeavesFullScreen)
  ///   Ctrl+Q  ferme l'application
  bool _onKey(KeyEvent event) {
    if (event is! KeyDownEvent) return false;
    final key = event.logicalKey;

    if (key == LogicalKeyboardKey.f11) {
      toggleFullScreen();
      return true;
    }
    if (HardwareKeyboard.instance.isControlPressed &&
        key == LogicalKeyboardKey.keyQ) {
      windowManager.close();
      return true;
    }
    if (escapeLeavesFullScreen && key == LogicalKeyboardKey.escape) {
      leaveFullScreen(); // pas consommé : Échap reste utilisable comme « Retour »
    }
    return false;
  }

  @override
  Widget build(BuildContext context) => ValueListenableBuilder<Locale>(
        valueListenable: appLocale,
        builder: (context, locale, _) => MaterialApp(
          title: 'GloubiTV',
          debugShowCheckedModeBanner: false,
          theme: AppTheme.dark(),
          locale: locale,
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          builder: (context, child) => AppUiScope(
            settings: _settings,
            child: child ?? const SizedBox.shrink(),
          ),
          home: const PairingScreen(),
        ),
      );
}
