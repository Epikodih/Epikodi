import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';

/// Ligne de réglage : libellé (et aide optionnelle) à gauche, contrôle à droite.
class SettingRow extends StatelessWidget {
  const SettingRow(
      {super.key, required this.label, required this.child, this.hint});

  final String label;
  final String? hint;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.md,
        vertical: AppSpacing.sm,
      ),
      decoration: BoxDecoration(
        color: AppColors.surfaceHigh.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(AppRadius.lg),
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  label,
                  style: AppText.body.copyWith(fontWeight: FontWeight.w600),
                ),
                if (hint != null)
                  Text(
                    hint!,
                    style: AppText.body.copyWith(
                      color: AppColors.textMuted,
                      fontSize: (AppText.body.fontSize ?? 16) * 0.8,
                    ),
                  ),
              ],
            ),
          ),
          const SizedBox(width: AppSpacing.md),
          child,
        ],
      ),
    );
  }
}

/// Carte de réglage compacte sur une seule ligne : libellé à gauche, contrôle
/// à droite (réduit automatiquement si la place manque). Pensée pour être mise
/// côte à côte (Expanded) avec une autre SettingCard.
///
/// Le contrôle ne doit pas contenir d'Expanded/Flexible (largeur non bornée).
class SettingCard extends StatelessWidget {
  const SettingCard({super.key, required this.label, required this.child});

  final String label;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.md,
        vertical: AppSpacing.xs,
      ),
      decoration: BoxDecoration(
        color: AppColors.surfaceHigh.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(AppRadius.lg),
      ),
      child: Row(
        children: [
          Text(
            label,
            style: AppText.body.copyWith(
              fontWeight: FontWeight.w600,
              color: AppColors.textMuted,
            ),
          ),
          const SizedBox(width: AppSpacing.sm),
          Expanded(
            child: Align(
              alignment: Alignment.centerRight,
              child: FittedBox(
                fit: BoxFit.scaleDown,
                alignment: Alignment.centerRight,
                child: child,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
