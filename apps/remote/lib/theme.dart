import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';

/// Tailles de texte pour un téléphone tenu à la main. Les couleurs, le
/// dégradé et la mascotte viennent du design system, comme sur la TV.
abstract final class RemoteText {
  static const display = TextStyle(
    fontSize: 40,
    fontWeight: FontWeight.w700,
    color: AppColors.text,
    fontFeatures: [FontFeature.tabularFigures()],
  );
  static const headline = TextStyle(
      fontSize: 26, fontWeight: FontWeight.w700, color: AppColors.text);
  static const title = TextStyle(
      fontSize: 19, fontWeight: FontWeight.w600, color: AppColors.text);
  static const body =
      TextStyle(fontSize: 16, height: 1.4, color: AppColors.text);
  static const muted =
      TextStyle(fontSize: 15, height: 1.4, color: AppColors.textMuted);
}

/// Thème sombre de la TV, avec les tailles du téléphone.
ThemeData remoteTheme() {
  final base = AppTheme.dark();
  final border = OutlineInputBorder(
    borderRadius: BorderRadius.circular(AppRadius.md),
    borderSide: const BorderSide(color: AppColors.surfaceHigh, width: 2),
  );
  return base.copyWith(
    textTheme: base.textTheme.copyWith(
      displayLarge: RemoteText.display,
      headlineMedium: RemoteText.headline,
      titleLarge: RemoteText.title,
      titleMedium: RemoteText.title,
      bodyLarge: RemoteText.body,
      bodyMedium: RemoteText.body,
      labelLarge: RemoteText.title.copyWith(fontSize: 16),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: AppColors.surface,
      hintStyle: RemoteText.body.copyWith(color: AppColors.textMuted),
      contentPadding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.md, vertical: AppSpacing.sm + 2),
      border: border,
      enabledBorder: border,
      focusedBorder: border.copyWith(
          borderSide: const BorderSide(color: AppColors.primary, width: 2)),
      errorBorder: border.copyWith(
          borderSide: const BorderSide(color: AppColors.error, width: 2)),
      focusedErrorBorder: border.copyWith(
          borderSide: const BorderSide(color: AppColors.error, width: 2)),
      errorStyle: RemoteText.muted.copyWith(color: AppColors.error),
      errorMaxLines: 3,
    ),
    dialogTheme: const DialogThemeData(backgroundColor: AppColors.surface),
    progressIndicatorTheme:
        const ProgressIndicatorThemeData(color: AppColors.primary),
  );
}
