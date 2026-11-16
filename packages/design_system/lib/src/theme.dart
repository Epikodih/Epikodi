import 'package:flutter/material.dart';

import 'tokens.dart';

abstract final class AppTheme {
  /// Thème sombre (usage TV : écran sombre, texte large).
  static ThemeData dark() {
    const scheme = ColorScheme.dark(
      primary: AppColors.primary,
      onPrimary: AppColors.onPrimary,
      secondary: AppColors.primary,
      surface: AppColors.surface,
      onSurface: AppColors.text,
      error: AppColors.error,
    );

    return ThemeData(
      useMaterial3: true,
      brightness: Brightness.dark,
      colorScheme: scheme,
      scaffoldBackgroundColor: AppColors.background,
      focusColor: AppColors.focus,
      textTheme: const TextTheme(
        displayLarge: AppText.display,
        headlineMedium: AppText.headline,
        titleLarge: AppText.title,
        bodyLarge: AppText.body,
        bodyMedium: AppText.body,
        labelLarge: AppText.label,
      ),
    );
  }
}
