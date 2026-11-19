import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../l10n/app_localizations.dart';
import '../locale.dart';

/// Bouton FR / EN : passe la TV et le téléphone dans l'autre langue, sans
/// avoir à aller chercher le bouton de langue sur la TV avec les flèches.
class LanguageToggle extends StatelessWidget {
  const LanguageToggle({super.key, required this.onLanguage});

  /// Envoie la langue choisie à la TV (`fr` ou `en`).
  final ValueChanged<String> onLanguage;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final current = Localizations.localeOf(context).languageCode;
    final next = current == 'fr' ? 'en' : 'fr';
    return Semantics(
      button: true,
      label: l10n.languageToggle(next.toUpperCase()),
      excludeSemantics: true,
      child: Material(
        color: AppColors.surface,
        shape: const StadiumBorder(
            side: BorderSide(color: AppColors.surfaceHigh, width: 2)),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          key: const ValueKey('language-toggle'),
          onTap: () {
            HapticFeedback.selectionClick();
            remoteLocale.value = Locale(next);
            onLanguage(next);
          },
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(Icons.translate_rounded,
                    size: 20, color: AppColors.textMuted),
                const SizedBox(width: 6),
                Text(
                  current.toUpperCase(),
                  style: const TextStyle(
                    fontSize: 15,
                    fontWeight: FontWeight.w700,
                    color: AppColors.text,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
