import 'dart:ui';

import 'package:flutter/foundation.dart';

import 'l10n/app_localizations.dart';

/// Langue courante de l'app. Au démarrage (pas encore de profil) : langue du
/// système si on la gère, sinon français. Plus tard, l'écran des comptes
/// fera `appLocale.value = Locale(profil['language'])`.
final appLocale = ValueNotifier<Locale>(_initialLocale());

Locale _initialLocale() {
  final sys = PlatformDispatcher.instance.locale.languageCode;
  final supported =
      AppLocalizations.supportedLocales.any((l) => l.languageCode == sys);
  return supported ? Locale(sys) : const Locale('fr');
}
