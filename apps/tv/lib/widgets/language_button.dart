import 'package:epikodi_design_system/epikodi_design_system.dart';
import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../locale.dart';

/// Sélecteur de langue réutilisable : une pilule « FR | EN ».
/// Une pastille dégradée glisse sous la langue active. Un clic (OK
/// télécommande) passe à la suivante. S'adapte aux langues de
/// AppLocalizations.supportedLocales.
///
/// [height] : hauteur imposée (pour s'aligner sur un autre bouton).
class LanguageButton extends StatelessWidget {
  const LanguageButton({super.key, this.autofocus = false, this.height});

  final bool autofocus;
  final double? height;

  static const _duration = Duration(milliseconds: 280);
  static const _padding = 4.0;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final locales = AppLocalizations.supportedLocales;
    final current = Localizations.localeOf(context).languageCode;
    final reduced = UiSettingsScope.of(context).reducedMotion;
    final duration = reduced ? Duration.zero : _duration;

    final index = locales
        .indexWhere((l) => l.languageCode == current)
        .clamp(0, locales.length - 1);

    void switchToNext() {
      appLocale.value = locales[(index + 1) % locales.length];
    }

    // Position de la pastille : -1 (premier segment) … 1 (dernier).
    final align =
        locales.length < 2 ? 0.0 : -1 + 2 * index / (locales.length - 1);

    final sized = height != null;

    Widget row = Row(
      crossAxisAlignment:
          sized ? CrossAxisAlignment.stretch : CrossAxisAlignment.center,
      children: [
        for (final l in locales)
          Expanded(
            child: _Segment(
              label: l.languageCode.toUpperCase(),
              selected: l.languageCode == current,
              duration: duration,
              large: sized,
            ),
          ),
      ],
    );
    if (sized) row = SizedBox(height: height! - 2 * _padding, child: row);

    return FocusableCard(
      autofocus: autofocus,
      borderRadius: 999,
      onTap: switchToNext,
      semanticLabel: l10n.switchLanguageLabel,
      child: Container(
        padding: const EdgeInsets.all(_padding),
        color: AppColors.surface,
        child: IntrinsicWidth(
          child: Stack(
            children: [
              // Pastille qui glisse, de la largeur d'un segment.
              Positioned.fill(
                child: AnimatedAlign(
                  duration: duration,
                  curve: Curves.easeOutCubic,
                  alignment: Alignment(align, 0),
                  child: FractionallySizedBox(
                    widthFactor: 1 / locales.length,
                    heightFactor: 1,
                    child: const DecoratedBox(
                      decoration: BoxDecoration(
                        gradient: AppBrand.gradient,
                        borderRadius: BorderRadius.all(Radius.circular(999)),
                      ),
                    ),
                  ),
                ),
              ),
              row,
            ],
          ),
        ),
      ),
    );
  }
}

class _Segment extends StatelessWidget {
  const _Segment({
    required this.label,
    required this.selected,
    required this.duration,
    required this.large,
  });

  final String label;
  final bool selected;
  final Duration duration;

  /// Version « grande » : même texte que le bouton « Nouveau code ».
  final bool large;

  @override
  Widget build(BuildContext context) {
    final base = large ? AppText.body : AppText.label;
    return Padding(
      padding: EdgeInsets.symmetric(
        horizontal: large ? AppSpacing.lg : AppSpacing.md,
        vertical: large ? 0 : AppSpacing.xs,
      ),
      child: Center(
        child: AnimatedDefaultTextStyle(
          duration: duration,
          curve: Curves.easeOut,
          style: base.copyWith(
            color: selected ? Colors.white : AppColors.textMuted,
            fontWeight: selected ? FontWeight.w700 : FontWeight.w500,
          ),
          child: Text(label),
        ),
      ),
    );
  }
}
