import 'package:flutter/widgets.dart';

/// Langue choisie avec le bouton FR / EN de la télécommande.
/// `null` : langue du téléphone (français par défaut).
final remoteLocale = ValueNotifier<Locale?>(null);
