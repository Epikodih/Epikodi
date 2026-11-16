import 'package:flutter/painting.dart';

/// Contrastes vérifiés (WCAG 2.2 AA, 4,5:1 pour le texte courant) :
/// text / textMuted / error sur background sont tous au-dessus de 4,5:1.
abstract final class AppColors {
  static const background = Color(0xFF0E1116);
  static const surface = Color(0xFF181C23);
  static const surfaceHigh = Color(0xFF232833);
  static const primary = Color(0xFF4F8CFF);
  static const onPrimary = Color(0xFFFFFFFF);
  static const text = Color(0xFFF2F4F8);
  static const textMuted = Color(0xFFB4BCCA);
  static const error = Color(0xFFFF7A7A);
  static const focus = Color(0xFFFFFFFF); // contour de focus, très visible
}

abstract final class AppSpacing {
  static const xs = 8.0;
  static const sm = 16.0;
  static const md = 24.0;
  static const lg = 32.0;
  static const xl = 48.0;

  /// Marges de sécurité TV (~5 % de l'écran).
  static const overscanH = 48.0;
  static const overscanV = 27.0;
}

abstract final class AppRadius {
  static const sm = 8.0;
  static const md = 12.0;
  static const lg = 20.0;
}

abstract final class AppDurations {
  static const fast = Duration(milliseconds: 150);
  static const normal = Duration(milliseconds: 300);
}

/// Taille de focus : contour épais, visible à 3 mètres.
abstract final class AppFocus {
  static const borderWidth = 4.0;
  static const scale = 1.06;
}

/// Tailles « 10 pieds » : lisibles à 3 mètres d'un téléviseur.
/// Elles sont multipliées par UiSettings.textScale (réglable pour tout le projet).
abstract final class AppText {
  static const display = TextStyle(fontSize: 96, fontWeight: FontWeight.w700, letterSpacing: 8, color: AppColors.text);
  static const headline = TextStyle(fontSize: 56, fontWeight: FontWeight.w700, color: AppColors.text);
  static const title = TextStyle(fontSize: 40, fontWeight: FontWeight.w600, color: AppColors.text);
  static const body = TextStyle(fontSize: 32, color: AppColors.text);
  static const label = TextStyle(fontSize: 28, fontWeight: FontWeight.w500, color: AppColors.textMuted);
}